#!/usr/bin/env python3
"""Heroes Lore: Wind of Soltia per-node crosswalk wrapper.

The generic gate lives in ``tools/ast/`` (adopted verbatim from the j2me home
``_template/``): ``JavaAstAuditDump.java`` emits the canonical javac AST + node
inventory, ``j2me-ast-audit`` emits the ``syn`` AST + node inventory, and
``validate_crosswalk.py`` enforces that every Java node and every Rust node
carries its own explicit decision. Those three are game-agnostic and untouched.

This wrapper is the WoS-specific part the README calls the "per-game wrapper". It
is responsible for:

* selecting the exact source/Rust items (the transliterated classes, below);
* running both live emitters and a classfile reader (``javap``);
* recomputing every locked digest so evidence and manifest agree;
* on the DEFAULT (validate) path: regenerating the git-ignored ``evidence`` table
  ONLY, then validating the TRACKED, hand-maintained ``manifest`` (schema 2,
  committed at ``tools/ast/wos.manifest.toml``) against that fresh evidence and
  printing the generic verifier's ``--coverage`` report. Authored op/adapt
  decisions live in that tracked manifest and PERSIST — this path never rewrites
  it;
* on ``--refresh-locks``: recomputing every lock from fresh evidence and
  (re)writing the tracked manifest while PRESERVING every authored decision
  (bootstrapping a zero-decision baseline the first time). Locks are therefore
  never hand-typed, and the generic verifier re-compares them each run.

Body granularity
----------------
A manifest "body" here is one transliterated **class**: its whole Java
translation unit (every method / field / initializer inventory, concatenated in
javac emission order) paired against every production item its Rust module emits
(one ``rust`` target per ``fn`` / ``impl`` fn / ``const`` / ``struct`` field /
…). The generic schema is happy with either granularity; class-level keeps the
baseline burn-down readable (one line per class) while the blanket cap (48) still
forces any future ``op`` to decompose into atomic steps. A transliteration lane
authoring decisions later references node index ranges *within* a class body, so
per-method reasoning is expressed as several ``op`` steps inside the class body.

Bytecode locks
--------------
``code_sha256`` / ``opcode_sha256`` are digests of the **named-Java crosswalk
oracle's** compiled ``Code`` attribute (``target/java-classes`` from
``just build-java``), read back with ``javap -p -c``. That oracle is exactly the
readable body the Rust is validated against, so a change to it must break the
lock. Binding these to the *original obfuscated* baseline ``.class`` bodies
instead would additionally require the JADX de-obfuscation name map (obfuscated
``an.class`` ↔ named ``Adler32``); that is a later refinement and is called out
here so the provenance of these two digests is never misread.

Everything this writes lands under git-ignored ``_reference/ast/`` (playbook R1);
the only tracked artifacts are this wrapper, the generic gate, and the recipes.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent  # repo root (…/tools/ast → repo)
AST_DIR = HERE
OUT_DIR = ROOT / "_reference" / "ast"

sys.path.insert(0, str(AST_DIR))
import validate_crosswalk as vc  # noqa: E402


# --- The transliterated corpus: (Java source, Rust module) pairs -------------
# Each class that currently has BOTH a named-Java body and a transliterated Rust
# body. Host-only Rust modules (game.rs, resources.rs, lib.rs) have no Java class
# and are intentionally excluded.
@dataclass(frozen=True)
class ClassPair:
    name: str  # display / body key suffix
    pkg: str  # compiled-class package dir under target/java-classes
    java: str  # repo-relative .java path
    rust: str  # repo-relative .rs path


CORPUS: tuple[ClassPair, ...] = (
    ClassPair("GameMIDlet", "rpg", "java/src/main/java/rpg/GameMIDlet.java", "transliteration/game-xlat/src/game_midlet.rs"),
    ClassPair("GameLoop", "defpackage", "java/src/main/java/defpackage/GameLoop.java", "transliteration/game-xlat/src/game_loop.rs"),
    ClassPair("GameState", "defpackage", "java/src/main/java/defpackage/GameState.java", "transliteration/game-xlat/src/game_state.rs"),
    ClassPair("BaseCanvas", "defpackage", "java/src/main/java/defpackage/BaseCanvas.java", "transliteration/game-xlat/src/base_canvas.rs"),
    ClassPair("TitleScreen", "defpackage", "java/src/main/java/defpackage/TitleScreen.java", "transliteration/game-xlat/src/title_screen.rs"),
    ClassPair("AssetCache", "defpackage", "java/src/main/java/defpackage/AssetCache.java", "transliteration/game-xlat/src/asset_cache.rs"),
    ClassPair("PngMerger", "defpackage", "java/src/main/java/defpackage/PngMerger.java", "transliteration/game-xlat/src/png_merger.rs"),
    ClassPair("ByteUtil", "defpackage", "java/src/main/java/defpackage/ByteUtil.java", "transliteration/game-xlat/src/byte_util.rs"),
    ClassPair("Adler32", "defpackage", "java/src/main/java/defpackage/Adler32.java", "transliteration/game-xlat/src/adler32.rs"),
    ClassPair("Crc32", "defpackage", "java/src/main/java/defpackage/Crc32.java", "transliteration/game-xlat/src/crc32.rs"),
    ClassPair("BitmapFont", "defpackage", "java/src/main/java/defpackage/BitmapFont.java", "transliteration/game-xlat/src/bitmap_font.rs"),
    ClassPair("FontManager", "defpackage", "java/src/main/java/defpackage/FontManager.java", "transliteration/game-xlat/src/font_manager.rs"),
    ClassPair("StringTable", "defpackage", "java/src/main/java/defpackage/StringTable.java", "transliteration/game-xlat/src/string_table.rs"),
    ClassPair("WrapFont", "defpackage", "java/src/main/java/defpackage/WrapFont.java", "transliteration/game-xlat/src/wrap_font.rs"),
    ClassPair("Menu", "defpackage", "java/src/main/java/defpackage/Menu.java", "transliteration/game-xlat/src/menu.rs"),
    ClassPair("MainMenu", "defpackage", "java/src/main/java/defpackage/MainMenu.java", "transliteration/game-xlat/src/main_menu.rs"),
    ClassPair("GameScreen", "defpackage", "java/src/main/java/defpackage/GameScreen.java", "transliteration/game-xlat/src/game_screen.rs"),
    ClassPair("SaveCipher", "defpackage", "java/src/main/java/defpackage/SaveCipher.java", "transliteration/game-xlat/src/save_cipher.rs"),
    ClassPair("RmsFile", "defpackage", "java/src/main/java/defpackage/RmsFile.java", "transliteration/game-xlat/src/rms_file.rs"),
    ClassPair("Directions", "defpackage", "java/src/main/java/defpackage/Directions.java", "transliteration/game-xlat/src/directions.rs"),
    ClassPair("AppConfig", "defpackage", "java/src/main/java/defpackage/AppConfig.java", "transliteration/game-xlat/src/app_config.rs"),
    ClassPair("Debug", "defpackage", "java/src/main/java/defpackage/Debug.java", "transliteration/game-xlat/src/debug.rs"),
    ClassPair("EntityList", "defpackage", "java/src/main/java/defpackage/EntityList.java", "transliteration/game-xlat/src/entity_list.rs"),
    ClassPair("Item", "defpackage", "java/src/main/java/defpackage/Item.java", "transliteration/game-xlat/src/item.rs"),
    ClassPair("Equipment", "defpackage", "java/src/main/java/defpackage/Equipment.java", "transliteration/game-xlat/src/equipment.rs"),
    ClassPair("Armor", "defpackage", "java/src/main/java/defpackage/Armor.java", "transliteration/game-xlat/src/armor.rs"),
    ClassPair("Weapon", "defpackage", "java/src/main/java/defpackage/Weapon.java", "transliteration/game-xlat/src/weapon.rs"),
    ClassPair("ItemBag", "defpackage", "java/src/main/java/defpackage/ItemBag.java", "transliteration/game-xlat/src/item_bag.rs"),
    ClassPair("AudioManager", "defpackage", "java/src/main/java/defpackage/AudioManager.java", "transliteration/game-xlat/src/audio_manager.rs"),
    ClassPair("SoundPlayer", "defpackage", "java/src/main/java/defpackage/SoundPlayer.java", "transliteration/game-xlat/src/sound_player.rs"),
)

# The full baseline class count of the game (89 default-package + rpg.GameMIDlet),
# so the coverage header can honestly frame "23 of the whole game under audit".
TOTAL_GAME_CLASSES = 90

OPCODE_LINE = re.compile(r"^\s*\d+:\s+(\S+)")


def sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def toml_str(value: str) -> str:
    """A valid TOML basic string (tabs are legal literally; escape the rest)."""
    out = []
    for ch in value:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 0x20 or ord(ch) == 0x7F:
            out.append(f"\\u{ord(ch):04x}")
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def toml_list(values: list[str], indent: str = "  ") -> str:
    if not values:
        return "[]"
    body = "".join(f"{indent}{toml_str(v)},\n" for v in values)
    return "[\n" + body + "]"


# --- Live emitters -----------------------------------------------------------
@dataclass
class Emitted:
    item: str
    ast: str
    nodes: list[str]


def _decode_nodes(raw: str) -> list[str]:
    # An empty inventory encodes to the empty string; it must yield ZERO nodes,
    # not one spurious empty node.
    return raw.split("\n") if raw else []


def run_java_emitter(emitter_classdir: Path, sources: list[Path]) -> dict[str, list[Emitted]]:
    """basename(.java) -> [Emitted] via JavaAstAuditDump (base64 columns)."""
    proc = subprocess.run(
        ["java", "-cp", str(emitter_classdir), "JavaAstAuditDump", *map(str, sources)],
        check=True,
        capture_output=True,
        text=True,
    )
    by_file: dict[str, list[Emitted]] = {}
    for line in proc.stdout.splitlines():
        if not line:
            continue
        parts = line.split("\t")
        # source, owner, key, base64(ast), base64(nodes)
        source, _owner, key, ast_b64, nodes_b64 = (parts + ["", "", "", "", ""])[:5]
        ast = base64.b64decode(ast_b64).decode("utf-8") if ast_b64 else ""
        nodes_raw = base64.b64decode(nodes_b64).decode("utf-8") if nodes_b64 else ""
        by_file.setdefault(Path(source).name, []).append(
            Emitted(item=key, ast=ast, nodes=_decode_nodes(nodes_raw))
        )
    return by_file


def run_rust_emitter(binary: Path, sources: list[Path]) -> dict[str, list[Emitted]]:
    """basename(.rs) -> [Emitted] via j2me-ast-audit --production-only (hex columns)."""
    proc = subprocess.run(
        [str(binary), "--production-only", *map(str, sources)],
        check=True,
        capture_output=True,
        text=True,
    )
    by_file: dict[str, list[Emitted]] = {}
    for line in proc.stdout.splitlines():
        if not line:
            continue
        parts = line.split("\t")
        # file, item, hex(ast), hex(nodes)
        file, item, ast_hex, nodes_hex = (parts + ["", "", "", ""])[:4]
        ast = bytes.fromhex(ast_hex).decode("utf-8") if ast_hex else ""
        nodes_raw = bytes.fromhex(nodes_hex).decode("utf-8") if nodes_hex else ""
        by_file.setdefault(Path(file).name, []).append(
            Emitted(item=item, ast=ast, nodes=_decode_nodes(nodes_raw))
        )
    return by_file


def javap_digests(class_dir: Path, pkg: str, name: str) -> tuple[str, str]:
    """(code_sha256, opcode_sha256) over the class + its nested classes."""
    pkg_dir = class_dir / pkg
    class_files = sorted(pkg_dir.glob(f"{name}.class")) + sorted(
        pkg_dir.glob(f"{name}$*.class")
    )
    if not class_files:
        raise SystemExit(
            f"missing compiled bytecode for {pkg}.{name} under {pkg_dir} — "
            f"run `just build-java` first"
        )
    proc = subprocess.run(
        ["javap", "-p", "-c", *map(str, class_files)],
        check=True,
        capture_output=True,
        text=True,
    )
    code = proc.stdout
    opcodes = [m.group(1) for m in map(OPCODE_LINE.match, code.splitlines()) if m]
    return sha(code), sha("\n".join(opcodes))


# --- Evidence + baseline manifest --------------------------------------------
@dataclass
class Body:
    java_item: str
    code_sha256: str
    opcode_sha256: str
    java_ast_sha256: str
    java_nodes: list[str]
    rust: list[Emitted]


def build_bodies(
    emitter_classdir: Path, rust_bin: Path, class_dir: Path
) -> list[Body]:
    java_sources = [ROOT / pair.java for pair in CORPUS]
    rust_sources = [ROOT / pair.rust for pair in CORPUS]
    java_by_file = run_java_emitter(emitter_classdir, java_sources)
    rust_by_file = run_rust_emitter(rust_bin, rust_sources)

    bodies: list[Body] = []
    for pair in CORPUS:
        j_items = java_by_file.get(Path(pair.java).name)
        r_items = rust_by_file.get(Path(pair.rust).name)
        if j_items is None:
            raise SystemExit(f"no javac AST emitted for {pair.java}")
        if r_items is None:
            raise SystemExit(f"no syn AST emitted for {pair.rust}")
        java_nodes: list[str] = []
        for item in j_items:
            java_nodes.extend(item.nodes)
        # A stable, formatting-independent digest of the whole class's javac AST.
        java_ast_sha256 = sha(
            "\n".join(f"{item.item}\x00{item.ast}" for item in j_items)
        )
        code_sha256, opcode_sha256 = javap_digests(class_dir, pair.pkg, pair.name)
        bodies.append(
            Body(
                java_item=f"{pair.pkg}.{pair.name}",
                code_sha256=code_sha256,
                opcode_sha256=opcode_sha256,
                java_ast_sha256=java_ast_sha256,
                java_nodes=java_nodes,
                rust=r_items,
            )
        )
    return bodies


def emit_evidence(bodies: list[Body]) -> str:
    lines = [
        "# GENERATED by tools/ast/wos_crosswalk.py — live emitter evidence.",
        "# JavaAstAuditDump (javac) + j2me-ast-audit (syn) + javap bytecode.",
        "",
    ]
    for body in bodies:
        lines += [
            "[[body]]",
            f"java_item = {toml_str(body.java_item)}",
            f"code_sha256 = {toml_str(body.code_sha256)}",
            f"opcode_sha256 = {toml_str(body.opcode_sha256)}",
            f"java_ast_sha256 = {toml_str(body.java_ast_sha256)}",
            f"java_nodes = {toml_list(body.java_nodes)}",
            "",
        ]
        for target in body.rust:
            ast_sha256 = sha(target.ast)
            lines += [
                "[[body.rust]]",
                f"file = {toml_str(next(p.rust for p in CORPUS if f'{p.pkg}.{p.name}' == body.java_item))}",
                f"item = {toml_str(target.item)}",
                f"ast_sha256 = {toml_str(ast_sha256)}",
                f"nodes = {toml_list(target.nodes)}",
                "",
            ]
    return "\n".join(lines) + "\n"


BASELINE_REVIEW = (
    "BASELINE (pre-decision): this class was transliterated before per-node "
    "op/adapt decisions existed. Zero nodes are decided here — every Java and "
    "Rust node is UNCHECKED at the node level, pending atomic-step authoring. "
    "This is the honest starting point, not a passing crosswalk."
)

DEFAULT_BUILD = "heroes-lore-wind-of-soltia v2.0.7 named-Java oracle"

# The TRACKED, hand-maintained manifest (committed, like the shipped fixture):
# it carries the reviewer's authored op/adapt/review/semantic_status decisions
# AND the locked digests. The locks are RECOMPUTED from live evidence by
# `--refresh-locks` (never hand-typed) and re-COMPARED by the generic verifier on
# every run; the decisions PERSIST because the default (validate) path never
# rewrites this file.
MANIFEST_PATH = AST_DIR / "wos.manifest.toml"
# The live emitter evidence, git-ignored (playbook R1). Regenerated EVERY run.
EVIDENCE_PATH = OUT_DIR / "wos.evidence.toml"


# --- Minimal, generic TOML serializer (stdlib has readers only) --------------
def _toml_scalar(value: object) -> str:
    if isinstance(value, bool):  # bool is an int subclass — check it first
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        return toml_str(value)
    raise TypeError(f"unserializable TOML scalar: {value!r}")


def _toml_inline(value: object) -> str:
    """Serialize a value as an inline TOML fragment (tables/arrays/scalars)."""
    if isinstance(value, dict):
        inner = ", ".join(f"{k} = {_toml_inline(v)}" for k, v in value.items())
        return "{ " + inner + " }" if inner else "{}"
    if isinstance(value, (list, tuple)):
        return "[" + ", ".join(_toml_inline(v) for v in value) + "]"
    return _toml_scalar(value)


# Canonical per-body scalar key order for stable diffs across refreshes/edits.
_BODY_SCALAR_ORDER = (
    "java_item",
    "code_sha256",
    "opcode_sha256",
    "java_ast_sha256",
    "java_nodes_sha256",
    "java_node_count",
    "semantic_status",
    "review",
)


def _serialize_array(name: str, entries: list) -> list[str]:
    if not entries:
        return [f"{name} = []"]
    lines = [f"{name} = ["]
    for entry in entries:
        lines.append(f"  {_toml_inline(entry)},")
    lines.append("]")
    return lines


def _serialize_body(body: dict) -> list[str]:
    lines = ["[[body]]"]
    for key in _BODY_SCALAR_ORDER:
        if body.get(key) is not None:
            lines.append(f"{key} = {_toml_scalar(body[key])}")
    lines += _serialize_array("rust", list(body.get("rust", [])))
    lines += _serialize_array("op", list(body.get("op", [])))
    lines += _serialize_array("adapt", list(body.get("adapt", [])))
    return lines


def serialize_manifest(manifest: dict) -> str:
    lines = [
        "# TRACKED, hand-maintained per-node crosswalk manifest (schema 2).",
        "#",
        "# op/adapt/review/semantic_status are the reviewer's AUTHORED decisions",
        "# and persist across runs — the default `wos_crosswalk.py` (validate) path",
        "# regenerates EVIDENCE only and never rewrites this file.",
        "#",
        "# The locks (code/opcode/AST/node-inventory digests + node counts) are",
        "# RECOMPUTED from live evidence by `wos_crosswalk.py --refresh-locks` (which",
        "# preserves every decision) and never hand-typed; the generic verifier",
        "# re-compares them against fresh evidence on every run, so a one-node drift",
        "# in either body breaks a lock.",
        "schema_version = 2",
        f"build = {toml_str(str(manifest.get('build', DEFAULT_BUILD)))}",
        f"total_body_count = {int(manifest['total_body_count'])}",
        f"reviewed_body_count = {int(manifest['reviewed_body_count'])}",
        f"crosswalked_body_count = {int(manifest['crosswalked_body_count'])}",
        "",
        "[policy]",
        f"blanket_max_span = {int(manifest.get('policy', {}).get('blanket_max_span', 48))}",
        "",
    ]
    for body in manifest["body"]:
        lines += _serialize_body(body)
        lines.append("")
    return "\n".join(lines) + "\n"


def refresh_manifest(bodies: list[Body], existing: dict | None) -> dict:
    """Recompute every lock from live evidence, PRESERVING authored decisions.

    ``existing`` is the parsed tracked manifest (or ``None`` to bootstrap a
    fresh baseline). op/adapt/review/semantic_status are carried over verbatim
    per body; only the digests, node counts, and rust-target inventory are
    recomputed from ``bodies`` (the live evidence).
    """
    prior = {b["java_item"]: b for b in existing.get("body", [])} if existing else {}
    new_bodies: list[dict] = []
    crosswalked = 0
    for pair, body in zip(CORPUS, bodies):
        prev = prior.get(body.java_item, {})
        rust_targets = [
            {
                "file": pair.rust,
                "item": target.item,
                "ast_sha256": sha(target.ast),
                "nodes_sha256": vc.node_inventory_digest(target.nodes),
                "node_count": len(target.nodes),
            }
            for target in body.rust
        ]
        merged = {
            "java_item": body.java_item,
            "code_sha256": body.code_sha256,
            "opcode_sha256": body.opcode_sha256,
            "java_ast_sha256": body.java_ast_sha256,
            "java_nodes_sha256": vc.node_inventory_digest(body.java_nodes),
            "java_node_count": len(body.java_nodes),
            "review": prev.get("review", BASELINE_REVIEW),
            "rust": rust_targets,
            "op": list(prev.get("op", [])),
            "adapt": list(prev.get("adapt", [])),
        }
        status = prev.get("semantic_status")
        if status:
            merged["semantic_status"] = status
            if status == "crosswalked":
                crosswalked += 1
        new_bodies.append(merged)
    return {
        "build": (existing or {}).get("build", DEFAULT_BUILD),
        "total_body_count": TOTAL_GAME_CLASSES,
        "reviewed_body_count": len(new_bodies),
        "crosswalked_body_count": crosswalked,
        "policy": {
            "blanket_max_span": (existing or {}).get("policy", {}).get(
                "blanket_max_span", 48
            )
        },
        "body": new_bodies,
    }


def _regenerate_evidence(args: argparse.Namespace) -> None:
    """Run both live emitters + javap and (re)write the git-ignored evidence."""
    if not args.rust_emitter_bin.exists():
        raise SystemExit(
            f"missing rust emitter {args.rust_emitter_bin} — "
            f"run `cargo build -p j2me-ast-audit` first"
        )
    # Compile the generic javac dumper into a scratch dir (self-contained).
    emitter_classdir = OUT_DIR / "emitter-classes"
    emitter_classdir.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["javac", "-d", str(emitter_classdir), str(AST_DIR / "JavaAstAuditDump.java")],
        check=True,
    )
    bodies = build_bodies(emitter_classdir, args.rust_emitter_bin, args.class_dir)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    EVIDENCE_PATH.write_text(emit_evidence(bodies), encoding="utf-8")
    print(f"wrote {EVIDENCE_PATH.relative_to(ROOT)} (live emitter evidence)")
    args._bodies = bodies  # stash for --refresh-locks


def _load_evidence() -> dict:
    import tomllib

    return vc.load_evidence(tomllib.loads(EVIDENCE_PATH.read_text(encoding="utf-8")))


def _load_manifest() -> dict:
    import tomllib

    if not MANIFEST_PATH.exists():
        raise SystemExit(
            f"tracked manifest {MANIFEST_PATH.relative_to(ROOT)} is missing — "
            f"bootstrap it with `wos_crosswalk.py --refresh-locks`"
        )
    return tomllib.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--class-dir",
        type=Path,
        default=ROOT / "target" / "java-classes",
        help="compiled named-Java classes (from `just build-java`)",
    )
    parser.add_argument(
        "--rust-emitter-bin",
        type=Path,
        default=ROOT / "target" / "debug" / "j2me-ast-audit",
        help="the built j2me-ast-audit binary",
    )
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--coverage", action="store_true")
    parser.add_argument(
        "--emit-only",
        action="store_true",
        help="regenerate the git-ignored evidence only, then stop (no validate)",
    )
    parser.add_argument(
        "--refresh-locks",
        action="store_true",
        help=(
            "recompute every lock from fresh evidence and (re)write the TRACKED "
            "manifest, PRESERVING all authored op/adapt/review/semantic_status "
            "decisions; bootstraps the manifest if it does not yet exist"
        ),
    )
    args = parser.parse_args()

    _regenerate_evidence(args)
    if args.emit_only:
        return 0

    import tomllib

    if args.refresh_locks:
        existing = (
            tomllib.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
            if MANIFEST_PATH.exists()
            else None
        )
        refreshed = refresh_manifest(args._bodies, existing)
        MANIFEST_PATH.write_text(serialize_manifest(refreshed), encoding="utf-8")
        verb = "refreshed locks in" if existing else "bootstrapped"
        preserved = sum(
            1 for b in refreshed["body"] if b.get("op") or b.get("adapt")
        )
        print(
            f"{verb} {MANIFEST_PATH.relative_to(ROOT)} "
            f"({len(refreshed['body'])} bodies; {preserved} carry authored decisions)"
        )

    manifest = _load_manifest()
    evidence = _load_evidence()
    report = vc.validate(manifest, evidence, strict=args.strict)
    print(vc.format_coverage(report))
    if report.errors:
        print("---", file=sys.stderr)
        # A per-body "no op/adapt decisions" red IS the unchecked baseline signal;
        # a "--strict: N node(s) ... undecided" red is the same signal aggregated.
        # Anything else is a wiring/lock bug worth surfacing loudly.
        expected = ("no op/adapt decisions", "--strict:")
        undecided = [e for e in report.errors if any(x in e for x in expected)]
        other = [e for e in report.errors if not any(x in e for x in expected)]
        for error in other[:80]:
            print(error, file=sys.stderr)
        print(
            f"[gate] {len(undecided)} undecided-baseline red(s); "
            f"{len(other)} other error(s)",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
