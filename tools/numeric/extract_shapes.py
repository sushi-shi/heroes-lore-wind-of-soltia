#!/usr/bin/env python3
"""Numeric-shape opcode authority for the baseline bytecode (playbook R8).

The classic J2ME->Rust transliteration bug is a *silent numeric-type drift*: a
Java `int / int` (which truncates) is hand-ported as Rust float division, an
`i2b`/`i2s` narrowing is dropped, or a stray float op is introduced. None of
these change the shape of the *control* flow, so a structural diff misses them.

This tool builds the ground-truth defense: for every concrete method in the
baseline JAR it extracts the ORDERED sequence of purely-numeric JVM opcodes —
integer/long arithmetic, float/double arithmetic, every i2*/l2*/f2*/d2*
conversion, and `iinc`. That per-method "numeric shape" (keyed
`class.method:descriptor`) can then be asserted BEFORE a method is transliterated
and re-checked after, so any numeric-type drift trips a gate instead of shipping.

How the opcodes are read: `javap -c -p -s -classpath <jar> <class>` disassembles
each class (`-c`), including private members (`-p`), with each member's JVM type
descriptor (`-s`). We parse the disassembly's instruction column — every code
line is `<offset>: <mnemonic> [operands]` — and keep only mnemonics in the
numeric universe below, in order. The `-s` `descriptor:` line gives the exact JVM
descriptor for the key; `<init>`/`<clinit>` are recovered from the member header.

R1 (resource-free): the JAR is copyrighted and git-ignored; the DERIVED shapes
are emitted ONLY under git-ignored `_reference/`, regenerated on demand. This
tool and its `just` recipe are the sole tracked artifacts — mirroring how
`tools/originals/verify.py` is tracked while `_originals/` is not.

Modes:
  (default)                       extract all shapes, write _reference JSON, and
                                  print a non-vacuous summary (fails if no
                                  numeric-bearing method is found -> parse broke).
  --assert-shape KEY=op,op,...    assert one method's numeric shape; exit non-
                                  zero on mismatch (the R8 gate in anger).
  --self-test                     prove the gate can fail (playbook R3): a
                                  correct shape verifies and a corrupted one is
                                  detected. Exits 0 iff both hold.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import zipfile
from pathlib import Path

# tools/numeric/extract_shapes.py -> repo root is parents[2].
ROOT = Path(__file__).resolve().parents[2]
DEFAULT_JAR = ROOT / "_originals" / "Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar"
OUT = ROOT / "_reference" / "numeric_shapes.json"

# --- The numeric opcode universe (task-specified), grouped for reporting. -----
INT_ARITH = "iadd isub imul idiv irem ineg iand ior ixor ishl ishr iushr".split()
LONG_ARITH = "ladd lsub lmul ldiv lrem lneg land lor lxor lshl lshr lushr".split()
FLOAT_ARITH = "fadd fsub fmul fdiv frem fneg".split()
DOUBLE_ARITH = "dadd dsub dmul ddiv drem dneg".split()
CONVERSIONS = "i2l i2f i2d l2i l2f l2d f2i f2l f2d d2i d2l d2f i2b i2c i2s".split()
MISC = ["iinc"]
NUMERIC_OPS = set(INT_ARITH + LONG_ARITH + FLOAT_ARITH + DOUBLE_ARITH
                  + CONVERSIONS + MISC)

# --- The R8 risk subset: ops where a numeric-type drift could slip in. --------
# Any float/double arithmetic or int<->float/double conversion that appears (or
# is wrongly introduced) is a floating-point hazard; and integer div/rem is the
# exact `int/int`-truncation site that must NOT become float division in Rust.
FLOATY = set(FLOAT_ARITH + DOUBLE_ARITH)
FLOAT_CONV = {"i2f", "i2d", "l2f", "l2d", "f2i", "f2l", "f2d", "d2i", "d2l", "d2f"}
INT_DIVREM = {"idiv", "irem", "ldiv", "lrem"}
RISK_OPS = FLOATY | FLOAT_CONV | INT_DIVREM

# An instruction line: leading indent, `<offset>: <mnemonic> ...`.
INSTR_RE = re.compile(r"^\s+\d+:\s+(\S+)")
MODS = {"public", "private", "protected", "static", "final", "abstract",
        "synchronized", "native", "strictfp", "transient", "volatile"}


# --- javap driver ------------------------------------------------------------

def list_classes(jar: Path) -> list[str]:
    """Binary class names in the JAR (dotted), sorted for determinism."""
    with zipfile.ZipFile(jar) as zf:
        names = [n[:-len(".class")].replace("/", ".")
                 for n in zf.namelist() if n.endswith(".class")]
    return sorted(names)


def javap(jar: Path, class_name: str) -> str:
    cmd = ["javap", "-c", "-p", "-s", "-classpath", str(jar), class_name]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"javap failed for {class_name}: {proc.stderr.strip() or proc.stdout.strip()}")
    return proc.stdout


# --- disassembly parsing -----------------------------------------------------

def member_name(header: str, simple: str) -> str:
    """Recover the JVM member name from a javap source-form member header.

    `<clinit>` prints as `static {}`; a constructor prints as the simple class
    name preceded only by access modifiers (no return type); everything else is
    the identifier immediately before the argument list.
    """
    h = header.strip().rstrip(";").strip()
    if "static {}" in h:
        return "<clinit>"
    before = h.split("(", 1)[0]
    toks = before.split()
    name = toks[-1]
    preceding = toks[:-1]
    if name == simple and all(t in MODS for t in preceding):
        return "<init>"
    return name


def parse_class(class_name: str, text: str) -> dict[str, list[str]]:
    """Map each concrete method's key -> ordered list of numeric opcodes.

    A method is recorded iff it has a `Code:` block (abstract/native/interface
    methods and fields have none and are skipped). The shape may be empty — an
    empty shape is a real, assertable fact (no numeric op must be introduced).
    """
    simple = class_name.split(".")[-1].split("$")[-1]
    shapes: dict[str, list[str]] = {}
    header: str | None = None
    descriptor: str | None = None
    cur_key: str | None = None
    in_code = False

    for line in text.splitlines():
        m = INSTR_RE.match(line)
        if m:
            if in_code and cur_key is not None:
                mnem = m.group(1)
                if mnem in NUMERIC_OPS:
                    shapes[cur_key].append(mnem)
            continue
        stripped = line.strip()
        # Member header: indent exactly 2, ends with ';' (methods AND fields).
        if line[:2] == "  " and line[2:3] != " " and stripped.endswith(";"):
            header, descriptor, cur_key, in_code = stripped, None, None, False
            continue
        if stripped.startswith("descriptor:"):
            descriptor = stripped[len("descriptor:"):].strip()
            continue
        if stripped == "Code:" and header is not None and descriptor is not None:
            key = f"{class_name}.{member_name(header, simple)}:{descriptor}"
            shapes.setdefault(key, [])
            cur_key, in_code = key, True
            continue
        # Other lines (class header, blank, 'Exception table:' + rows) carry no
        # numeric instruction and never leak: the next member header resets.
    return shapes


def extract_all(jar: Path) -> tuple[dict[str, list[str]], int]:
    """Return ({key: [opcodes]}, n_classes) for every class in the JAR."""
    classes = list_classes(jar)
    shapes: dict[str, list[str]] = {}
    for cls in classes:
        shapes.update(parse_class(cls, javap(jar, cls)))
    return shapes, len(classes)


# --- assertion / self-test ---------------------------------------------------

def parse_ops(spec: str) -> list[str]:
    return [t for t in re.split(r"[,\s]+", spec.strip()) if t]


def compare_shape(shapes: dict[str, list[str]], key: str,
                  expected: list[str]) -> tuple[bool, str]:
    """(matches?, human message). Missing key is a non-match (exit-worthy)."""
    if key not in shapes:
        return False, f"no such method in baseline: {key}"
    actual = shapes[key]
    if actual == expected:
        return True, f"OK {key} == [{', '.join(actual) or '(none)'}]"
    return False, (f"MISMATCH {key}\n  expected: [{', '.join(expected) or '(none)'}]"
                   f"\n  actual:   [{', '.join(actual) or '(none)'}]")


def do_self_test(shapes: dict[str, list[str]]) -> int:
    """Prove the gate bites: a correct shape verifies, a corrupted one trips."""
    # Deterministically pick a meaningful, non-empty target (prefer an int/int
    # div/rem method — the headline R8 hazard); fall back to any numeric method.
    numeric = sorted(k for k, v in shapes.items() if v)
    if not numeric:
        print("SELF-TEST FAILED: no numeric-bearing method to exercise the gate "
              "(parser likely broke).")
        return 3
    target = next((k for k in numeric
                   if INT_DIVREM.intersection(shapes[k])), numeric[0])
    correct = shapes[target]
    corrupt = correct + ["fmul"]  # inject a float op that is provably absent

    ok_clean, msg_clean = compare_shape(shapes, target, correct)
    ok_dirty, _ = compare_shape(shapes, target, corrupt)
    if not ok_clean:
        print(f"SELF-TEST FAILED: the true shape did not verify:\n  {msg_clean}")
        return 3
    if ok_dirty:
        print("SELF-TEST FAILED: a corrupted shape was NOT detected (vacuous gate).")
        return 3
    print(f"self-test OK: the true numeric shape of {target} verifies, and a "
          f"one-op corruption (+fmul) of it is detected as a mismatch.")
    return 0


# --- reporting ---------------------------------------------------------------

def write_reference(jar: Path, shapes: dict[str, list[str]], n_classes: int) -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "generated_by": "tools/numeric/extract_shapes.py",
        "note": ("R1: derived from copyrighted bytecode; git-ignored under "
                 "_reference/; regenerate on demand, never commit."),
        "jar": str(jar.relative_to(ROOT)) if jar.is_relative_to(ROOT) else str(jar),
        "opcode_universe": sorted(NUMERIC_OPS),
        "classes": n_classes,
        "methods": len(shapes),
        "shapes": {k: shapes[k] for k in sorted(shapes)},
    }
    OUT.write_text(json.dumps(payload, indent=1, sort_keys=False) + "\n")


def print_summary(jar: Path, shapes: dict[str, list[str]], n_classes: int) -> int:
    n_methods = len(shapes)
    numeric = {k: v for k, v in shapes.items() if v}
    floaty = {k: v for k, v in shapes.items() if FLOATY.intersection(v)}
    fconv = {k: v for k, v in shapes.items() if FLOAT_CONV.intersection(v)}
    divrem = {k: v for k, v in shapes.items() if INT_DIVREM.intersection(v)}
    risk = {k: v for k, v in shapes.items() if RISK_OPS.intersection(v)}

    print(f"numeric-shape authority  <-  {jar.name}")
    print(f"  wrote {OUT.relative_to(ROOT)} (R1: git-ignored, regenerable)")
    print(f"  classes analyzed .................. {n_classes}")
    print(f"  concrete methods (with bytecode) .. {n_methods}")
    print(f"  methods with any numeric op ....... {len(numeric)}")
    print(f"  -- R8 transliteration risk spots (numeric-type-drift hazards) --")
    print(f"  float/double arithmetic methods ... {len(floaty)}")
    print(f"  int<->float/double conversion ..... {len(fconv)}")
    print(f"  integer div/rem (int/int trunc) ... {len(divrem)}")
    print(f"  RISK methods total (union) ........ {len(risk)}")

    if numeric:
        top = sorted(numeric.items(), key=lambda kv: (-len(kv[1]), kv[0]))[:8]
        print("  top numeric-shape methods (by op count):")
        for k, v in top:
            print(f"    {len(v):>3}  {k}  ->  [{', '.join(v[:12])}"
                  f"{', ...' if len(v) > 12 else ''}]")
    if risk:
        top_risk = sorted(risk.items(),
                          key=lambda kv: (-len(RISK_OPS.intersection(kv[1])), kv[0]))[:8]
        print("  top R8 risk methods (by #risk ops):")
        for k, v in top_risk:
            hits = [op for op in v if op in RISK_OPS]
            print(f"    {len(hits):>3}  {k}  ->  risk-ops [{', '.join(hits[:12])}"
                  f"{', ...' if len(hits) > 12 else ''}]")

    if not numeric:
        print("numeric-shape: FAIL — 0 methods carry any numeric opcode; the "
              "disassembly parse almost certainly broke (vacuous authority).",
              file=sys.stderr)
        return 1
    return 0


# --- main --------------------------------------------------------------------

def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Numeric-shape opcode authority (R8).")
    ap.add_argument("--jar", type=Path, default=DEFAULT_JAR,
                    help="baseline JAR to disassemble (default: v207 baseline).")
    ap.add_argument("--assert-shape", metavar="KEY=op,op,...",
                    help="assert one method's ordered numeric shape; non-zero on mismatch.")
    ap.add_argument("--self-test", action="store_true",
                    help="prove the gate can fail (playbook R3); exit 0 iff it does.")
    args = ap.parse_args(argv)

    jar: Path = args.jar
    if not jar.is_file():
        print(f"baseline JAR not found: {jar}\n"
              f"  materialize it with `just bootstrap <resources>` (R1).",
              file=sys.stderr)
        return 2

    shapes, n_classes = extract_all(jar)

    if args.self_test:
        return do_self_test(shapes)

    if args.assert_shape:
        if "=" not in args.assert_shape:
            print("--assert-shape needs KEY=op,op,...", file=sys.stderr)
            return 2
        key, spec = args.assert_shape.split("=", 1)
        ok, msg = compare_shape(shapes, key.strip(), parse_ops(spec))
        print(msg)
        return 0 if ok else 1

    write_reference(jar, shapes, n_classes)
    return print_summary(jar, shapes, n_classes)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
