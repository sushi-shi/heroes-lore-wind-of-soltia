package defpackage;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import javax.microedition.rms.RecordStore;

/* renamed from: au */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:au.class */
/**
 * A thin "XFile" wrapper over a J2ME {@link RecordStore} that presents it as a
 * single stream-like blob. The store name is the requested path with {@code '/'}
 * replaced by {@code '_'}. All writes are buffered into one
 * {@link #writeBuffer} and flushed by {@link #close} as a single record
 * (replacing the store's contents); reads pull the last record into a
 * {@link #readBuffer}. This is how the game persists its save slots and options
 * (see {@link GameState} save/load and {@link GameLoop} options): each logical
 * file is one record store holding one packed, {@link SaveCipher}-scrambled
 * record. Debug lines ("XFile", "write", "read", "available"...) are the
 * original developer traces, preserved.
 */
public final class RmsFile {
    /* renamed from: a */
    /** The backing record store (null if it could not be opened). */
    public RecordStore store;

    /* renamed from: a, reason: collision with other field name */
    /** True once the store opened successfully; {@link #size} throws otherwise. */
    public boolean open;

    /* renamed from: a, reason: collision with other field name */
    /** Store name: the requested path with {@code '/'} turned into {@code '_'}. */
    public String storeName;

    /* renamed from: a, reason: collision with other field name */
    /** Lazily-created input over the last record, feeding {@link #read}. */
    public ByteArrayInputStream readBuffer = null;

    /* renamed from: a, reason: collision with other field name */
    /** Lazily-created accumulator collecting {@link #write} calls until {@link #close}. */
    public ByteArrayOutputStream writeBuffer = null;

    /**
     * Opens the store for {@code path}. {@code mode == 1} is read mode (the store
     * must already exist, and a failure is rethrown); any other mode creates the
     * store if necessary and swallows failures (leaving {@link #open} false).
     */
    public RmsFile(String path, int mode) throws Exception {
        this.store = null;
        this.open = true;
        String name = path.replace('/', '_');
        this.storeName = name;
        try {
            System.out.println(new StringBuffer().append("XFile ").append(name).toString());
            this.store = RecordStore.openRecordStore(name, mode != 1);
            if (this.store == null) {
                throw new Exception("");
            }
        } catch (Exception e) {
            this.open = false;
            if (mode == 1) {
                throw e;
            }
        }
    }

    /* renamed from: a */
    /**
     * Flushes any buffered writes as the store's single record (clearing prior
     * records first) and closes the store. Errors are ignored — closing must not
     * throw.
     */
    public final void close() {
        if (this.writeBuffer != null) {
            try {
                if (this.store.getNumRecords() > 0) {
                    this.store.closeRecordStore();
                    RecordStore.deleteRecordStore(this.storeName);
                    this.store = RecordStore.openRecordStore(this.storeName, true);
                }
                byte[] payload = this.writeBuffer.toByteArray();
                System.out.println(new StringBuffer().append("close : ").append(payload.length).toString());
                this.store.addRecord(payload, 0, payload.length);
            } catch (Exception unused) {
            }
        }
        try {
            this.store.closeRecordStore();
        } catch (Exception unused2) {
        }
    }

    /* renamed from: a */
    /** Appends {@code length} bytes from {@code buffer} at {@code offset} to the write buffer. */
    public final void write(byte[] buffer, int offset, int length) throws Exception {
        System.out.println(new StringBuffer().append("write ").append(this.storeName).toString());
        if (this.writeBuffer == null) {
            this.writeBuffer = new ByteArrayOutputStream();
        }
        this.writeBuffer.write(buffer, offset, length);
    }

    /* renamed from: b */
    /**
     * Reads {@code length} bytes into {@code buffer} at {@code offset} from the
     * last record, lazily loading it into the read buffer on first use.
     */
    public final void read(byte[] buffer, int offset, int length) throws Exception {
        System.out.println(new StringBuffer().append("read ").append(this.storeName).toString());
        if (this.readBuffer == null) {
            this.readBuffer = new ByteArrayInputStream(this.store.getRecord(this.store.getNextRecordID() - 1));
        }
        this.readBuffer.read(buffer, offset, length);
    }

    /* renamed from: a, reason: collision with other method in class */
    /** Size in bytes of the last record; throws if the store never opened. */
    public final int size() throws Exception {
        System.out.println(new StringBuffer().append("available ").append(this.storeName).toString());
        if (this.open) {
            return this.store.getRecordSize(this.store.getNextRecordID() - 1);
        }
        System.out.println("available false");
        throw new Exception("");
    }

    /* renamed from: a */
    /** True if a record store exists for {@code path} (open-then-close probe). */
    public static final boolean exists(String path) {
        String name = path.replace('/', '_');
        System.out.println(new StringBuffer().append("exists ").append(name).toString());
        try {
            RecordStore.openRecordStore(name, false).closeRecordStore();
            return true;
        } catch (Exception unused) {
            System.out.println("exists false");
            return false;
        }
    }

    /* renamed from: a, reason: collision with other method in class */
    /** Deletes the record store for {@code path} if present (errors ignored). */
    public static final void delete(String path) {
        String name = path.replace('/', '_');
        System.out.println(new StringBuffer().append("unlink ").append(name).toString());
        try {
            RecordStore.deleteRecordStore(name);
        } catch (Exception unused) {
        }
    }
}
