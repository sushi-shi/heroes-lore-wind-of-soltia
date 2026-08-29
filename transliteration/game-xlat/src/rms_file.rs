//! Transliterated from `java/src/main/java/defpackage/RmsFile.java`
//! (original `au.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A thin "XFile" wrapper over a J2ME `RecordStore` that presents it as a single
//! stream-like blob. The store name is the requested path with `'/'` replaced by
//! `'_'`. All writes are buffered into one [`ByteArrayOutputStream`] and flushed
//! by [`close`] as a single record (replacing the store's contents); reads pull
//! the last record into a [`ByteArrayInputStream`]. This is how the game persists
//! its save slots and options.
//!
//! An **instance** class (no `static` fields → no `ownership.tsv` rows): each
//! logical file is one [`RmsFileState`], owned by the save/options code that
//! opens it (`GameState`/`GameLoop`, not yet ported). The backing MIDP namespace
//! is the host-owned [`RmsRuntime`] (`crates/j2me-me`), threaded as `&mut` — the
//! same host-seam pattern the graphics/media runtimes use.
//!
//! The developer `System.out.println` traces ("XFile", "write", "read",
//! "available", "exists", "unlink", "close : n") are debug residue → dropped as
//! no-ops (`docs/TRANSLITERATION.md`, *No-ops*).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `au.<init>:(Ljava/lang/String;I)V => []`, `au.a:()V (close) => []`,
//! `au.a:([BII)V (write) => []`, `au.b:([BII)V (read) => ["isub"]`,
//! `au.a:()I (size) => ["isub"]`, `au.a:(Ljava/lang/String;)Z (exists) => []`,
//! `au.a:(Ljava/lang/String;)V (delete) => []`.

use j2me_jvm::JavaError;
use j2me_me::RmsRuntime;

/// A faithful stand-in for `java.io.ByteArrayOutputStream` — the `writeBuffer`
/// accumulator (only `write(byte[],int,int)` and `toByteArray()` are used).
#[derive(Debug, Default)]
pub struct ByteArrayOutputStream {
    buf: Vec<i8>,
}

impl ByteArrayOutputStream {
    /// `new ByteArrayOutputStream()`.
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// `write(byte[] b, int off, int len)` — appends `b[off .. off+len]`. The JDK
    /// throws `IndexOutOfBoundsException` on a bad range (reproduced, not a panic).
    fn write(&mut self, b: &[i8], off: i32, len: i32) -> Result<(), JavaError> {
        if off < 0 || len < 0 || (off as i64) + (len as i64) > b.len() as i64 {
            return Err(JavaError::ArrayIndexOutOfBounds {
                index: off.wrapping_add(len),
                length: b.len() as i32,
            });
        }
        self.buf
            .extend_from_slice(&b[off as usize..(off as usize) + (len as usize)]);
        Ok(())
    }

    /// `toByteArray()` — a copy of the accumulated bytes.
    fn to_byte_array(&self) -> Vec<i8> {
        self.buf.clone()
    }
}

/// A faithful stand-in for `java.io.ByteArrayInputStream` — the `readBuffer` over
/// the last record (only `read(byte[],int,int)` is used).
#[derive(Debug)]
pub struct ByteArrayInputStream {
    buf: Vec<i8>,
    pos: usize,
}

impl ByteArrayInputStream {
    /// `new ByteArrayInputStream(bytes)`.
    fn new(buf: Vec<i8>) -> Self {
        Self { buf, pos: 0 }
    }

    /// `read(byte[] b, int off, int len)` — copies up to `len` bytes into
    /// `b[off..]`, returning the count (or `-1` at EOF), advancing the cursor. The
    /// JDK bad-range throw is reproduced.
    fn read(&mut self, b: &mut [i8], off: i32, len: i32) -> Result<i32, JavaError> {
        if off < 0 || len < 0 || (off as i64) + (len as i64) > b.len() as i64 {
            return Err(JavaError::ArrayIndexOutOfBounds {
                index: off.wrapping_add(len),
                length: b.len() as i32,
            });
        }
        if self.pos >= self.buf.len() {
            return Ok(-1); // EOF
        }
        let avail = self.buf.len() - self.pos;
        let n = (len as usize).min(avail);
        if n == 0 {
            return Ok(0);
        }
        b[off as usize..(off as usize) + n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n as i32)
    }
}

/// Java `au` / `RmsFile` instance state (all fields are instance, not `static`).
#[derive(Debug)]
pub struct RmsFileState {
    /// `public RecordStore store;` — the backing store handle (its name in the
    /// [`RmsRuntime`] model); `None` when the store could not be opened.
    pub store: Option<String>,
    /// `public boolean open;` — true once the store opened successfully.
    pub open: bool,
    /// `public String storeName;` — the path with `'/'` turned into `'_'`.
    pub store_name: String,
    /// `public ByteArrayInputStream readBuffer;` — lazy input over the last record.
    pub read_buffer: Option<ByteArrayInputStream>,
    /// `public ByteArrayOutputStream writeBuffer;` — lazy accumulator for writes.
    pub write_buffer: Option<ByteArrayOutputStream>,
}

/// `RmsFile(String path, int mode) throws Exception` (`au.<init>`).
///
/// `mode == 1` is read mode (the store must already exist, and a failure is
/// rethrown); any other mode creates the store if necessary and swallows failures
/// (leaving `open` false).
pub fn new_rms_file(
    rms: &mut RmsRuntime,
    path: &str,
    mode: i32,
) -> Result<RmsFileState, JavaError> {
    // this.store = null; this.open = true;
    // String name = path.replace('/', '_'); this.storeName = name;
    let name = path.replace('/', "_");
    let mut state = RmsFileState {
        store: None,
        open: true,
        store_name: name.clone(),
        read_buffer: None,
        write_buffer: None,
    };
    // try { store = RecordStore.openRecordStore(name, mode != 1);
    //       if (store == null) throw new Exception(""); }
    match rms.open(&name, mode != 1) {
        Ok(handle) => {
            state.store = Some(handle);
        }
        Err(e) => {
            // catch (Exception e) { open = false; if (mode == 1) throw e; }
            state.open = false;
            if mode == 1 {
                return Err(e);
            }
        }
    }
    Ok(state)
}

/// `public final void close()` (`au.a:()V`). Flushes any buffered writes as the
/// store's single record (clearing prior records first) and closes the store.
/// Errors are ignored — closing must not throw.
pub fn close(s: &mut RmsFileState, rms: &mut RmsRuntime) {
    // if (writeBuffer != null) { try { ... } catch (Exception unused) {} }
    if s.write_buffer.is_some() {
        let _ = close_flush(s, rms);
    }
    // try { store.closeRecordStore(); } catch (Exception unused2) {}
    if let Some(name) = s.store.clone() {
        let _ = rms.close(&name);
    }
}

/// The guarded body of [`close`]'s first `try` block (all exceptions swallowed by
/// the caller). `store` being `null` (open failed) surfaces as [`JavaError::NullPointer`],
/// exactly the NPE the original would catch.
fn close_flush(s: &mut RmsFileState, rms: &mut RmsRuntime) -> Result<(), JavaError> {
    // if (store.getNumRecords() > 0)
    let store_name = s.store.clone().ok_or(JavaError::NullPointer)?;
    if rms.get(&store_name)?.num_records() > 0 {
        // store.closeRecordStore();
        rms.close(&store_name)?;
        // RecordStore.deleteRecordStore(storeName);
        rms.delete_store(&s.store_name)?;
        // store = RecordStore.openRecordStore(storeName, true);
        let handle = rms.open(&s.store_name, true)?;
        s.store = Some(handle);
    }
    // byte[] payload = writeBuffer.toByteArray();
    let payload = s
        .write_buffer
        .as_ref()
        .ok_or(JavaError::NullPointer)?
        .to_byte_array();
    // store.addRecord(payload, 0, payload.length);
    let store_name = s.store.clone().ok_or(JavaError::NullPointer)?;
    rms.get_mut(&store_name)?
        .add_record(&payload, 0, payload.len() as i32)?;
    Ok(())
}

/// `public final void write(byte[] buffer, int offset, int length) throws Exception`
/// (`au.a:([BII)V`). Appends `length` bytes from `buffer` at `offset` to the write
/// buffer.
pub fn write(
    s: &mut RmsFileState,
    buffer: &[i8],
    offset: i32,
    length: i32,
) -> Result<(), JavaError> {
    // if (writeBuffer == null) writeBuffer = new ByteArrayOutputStream();
    if s.write_buffer.is_none() {
        s.write_buffer = Some(ByteArrayOutputStream::new());
    }
    // writeBuffer.write(buffer, offset, length);
    s.write_buffer
        .as_mut()
        .expect("just created")
        .write(buffer, offset, length)
}

/// `public final void read(byte[] buffer, int offset, int length) throws Exception`
/// (`au.b:([BII)V`). Reads `length` bytes into `buffer` at `offset` from the last
/// record, lazily loading it on first use.
pub fn read(
    s: &mut RmsFileState,
    rms: &RmsRuntime,
    buffer: &mut [i8],
    offset: i32,
    length: i32,
) -> Result<(), JavaError> {
    // if (readBuffer == null)
    //   readBuffer = new ByteArrayInputStream(store.getRecord(store.getNextRecordID() - 1));
    if s.read_buffer.is_none() {
        let store_name = s.store.clone().ok_or(JavaError::NullPointer)?;
        let store = rms.get(&store_name)?;
        // getNextRecordID() - 1
        let id = store.next_record_id().wrapping_sub(1);
        let record = store.get_record(id)?;
        s.read_buffer = Some(ByteArrayInputStream::new(record));
    }
    // readBuffer.read(buffer, offset, length);  (return value discarded)
    s.read_buffer
        .as_mut()
        .expect("just created")
        .read(buffer, offset, length)?;
    Ok(())
}

/// `public final int size() throws Exception` (`au.a:()I`). Size in bytes of the
/// last record; throws if the store never opened.
pub fn size(s: &RmsFileState, rms: &RmsRuntime) -> Result<i32, JavaError> {
    if s.open {
        // return store.getRecordSize(store.getNextRecordID() - 1);
        let store_name = s.store.clone().ok_or(JavaError::NullPointer)?;
        let store = rms.get(&store_name)?;
        let id = store.next_record_id().wrapping_sub(1);
        return store.get_record_size(id);
    }
    // throw new Exception("")  — a generic checked Exception (no dedicated variant).
    Err(JavaError::Io(String::new()))
}

/// `public static final boolean exists(String path)` (`au.a:(Ljava/lang/String;)Z`).
/// True if a record store exists for `path` (open-then-close probe).
pub fn exists(rms: &mut RmsRuntime, path: &str) -> bool {
    // String name = path.replace('/', '_');
    let name = path.replace('/', "_");
    // try { openRecordStore(name, false).closeRecordStore(); return true; }
    // catch { return false; }
    match rms.open(&name, false) {
        Ok(handle) => {
            let _ = rms.close(&handle);
            true
        }
        Err(_) => false,
    }
}

/// `public static final void delete(String path)` (`au.a:(Ljava/lang/String;)V`).
/// Deletes the record store for `path` if present (errors ignored).
pub fn delete(rms: &mut RmsRuntime, path: &str) {
    // String name = path.replace('/', '_');
    let name = path.replace('/', "_");
    // try { RecordStore.deleteRecordStore(name); } catch {}
    let _ = rms.delete_store(&name);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real save/options round-trip: open a write-mode XFile, buffer bytes,
    /// close (flush one packed record), then read them back through a read-mode
    /// XFile. The store name derives from the path (`/` → `_`).
    #[test]
    fn write_close_read_round_trips_a_slot() {
        let mut rms = RmsRuntime::new();
        // Write mode (mode != 1) creates the store.
        let mut f = new_rms_file(&mut rms, "/save0", 0).expect("open write mode");
        assert!(f.open);
        assert_eq!(f.store_name, "_save0"); // '/' → '_'

        let payload: Vec<i8> = vec![1, 2, 3, -4, 5, 0, 127, -128];
        assert!(!payload.is_empty(), "count floor: something is written");
        write(&mut f, &payload, 0, payload.len() as i32).expect("write");
        close(&mut f, &mut rms);

        // The store now exists and holds exactly one packed record.
        assert!(exists(&mut rms, "/save0"));
        assert_eq!(rms.get("_save0").unwrap().num_records(), 1);

        // Read mode requires an existing store; size() + read() recover the bytes.
        let mut r = new_rms_file(&mut rms, "/save0", 1).expect("open read mode");
        let n = size(&r, &rms).expect("size");
        assert_eq!(n, payload.len() as i32);
        let mut buf = vec![0i8; n as usize];
        read(&mut r, &rms, &mut buf, 0, n).expect("read");
        assert_eq!(buf, payload);

        // Negative control (teeth): the read-back is the real payload, not a
        // one-byte perturbation of it.
        let mut mutated = payload.clone();
        mutated[3] = mutated[3].wrapping_add(1);
        assert_ne!(
            buf, mutated,
            "round-trip carried no data — the test is blind"
        );
    }

    /// `close` on a store that already has a record deletes+recreates it so exactly
    /// one packed record survives (the id monotonic reset the model reproduces).
    #[test]
    fn close_replaces_the_single_record() {
        let mut rms = RmsRuntime::new();
        let mut f = new_rms_file(&mut rms, "/opt", 0).unwrap();
        write(&mut f, &[9, 9, 9], 0, 3).unwrap();
        close(&mut f, &mut rms);
        assert_eq!(rms.get("_opt").unwrap().num_records(), 1);

        // Reopen, write different bytes, close: the prior record is replaced.
        let mut f = new_rms_file(&mut rms, "/opt", 0).unwrap();
        write(&mut f, &[1, 2], 0, 2).unwrap();
        close(&mut f, &mut rms);
        assert_eq!(
            rms.get("_opt").unwrap().num_records(),
            1,
            "still a single packed record after replace"
        );

        let mut r = new_rms_file(&mut rms, "/opt", 1).unwrap();
        let n = size(&r, &rms).unwrap();
        assert_eq!(n, 2);
        let mut buf = vec![0i8; 2];
        read(&mut r, &rms, &mut buf, 0, 2).unwrap();
        assert_eq!(buf, vec![1, 2]);
    }

    /// Read mode (`mode == 1`) on an absent store rethrows the open failure.
    #[test]
    fn read_mode_on_absent_store_throws() {
        let mut rms = RmsRuntime::new();
        assert!(new_rms_file(&mut rms, "/missing", 1).is_err());
        // A non-read mode instead swallows the failure but succeeds by creating.
        let f = new_rms_file(&mut rms, "/missing", 2).expect("create mode succeeds");
        assert!(f.open);
    }

    /// `exists`/`delete` probes: absent → false, present after close → true, gone
    /// after delete.
    #[test]
    fn exists_and_delete_probe_the_namespace() {
        let mut rms = RmsRuntime::new();
        assert!(!exists(&mut rms, "/gone"));

        let mut f = new_rms_file(&mut rms, "/gone", 0).unwrap();
        write(&mut f, &[42], 0, 1).unwrap();
        close(&mut f, &mut rms);
        assert!(exists(&mut rms, "/gone"));

        delete(&mut rms, "/gone");
        assert!(!exists(&mut rms, "/gone"));
    }

    /// `size` before the store ever opened throws (the `open == false` branch).
    #[test]
    fn size_on_unopened_store_throws() {
        // Force an unopened state directly (open() never succeeded).
        let rms = RmsRuntime::new();
        let f = RmsFileState {
            store: None,
            open: false,
            store_name: "_x".to_string(),
            read_buffer: None,
            write_buffer: None,
        };
        assert!(size(&f, &rms).is_err());
    }
}
