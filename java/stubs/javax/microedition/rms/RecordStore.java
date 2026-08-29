package javax.microedition.rms;

// Minimal compile-only stub of the MIDP RecordStore (JSR-118 rms).
// Behaviour-free. Method set matches exactly what the baseline references
// (au). Every method that touches records declares `throws RecordStoreException`
// to mirror the real API; au wraps all calls in try/catch(Exception) or
// `throws Exception`, so this compiles and preserves the checked-ness.
public class RecordStore {

    RecordStore() {
    }

    public static RecordStore openRecordStore(String recordStoreName, boolean createIfNecessary)
            throws RecordStoreException {
        return new RecordStore();
    }

    public static void deleteRecordStore(String recordStoreName)
            throws RecordStoreException {
    }

    public void closeRecordStore() throws RecordStoreException {
    }

    public int getNumRecords() throws RecordStoreException {
        return 0;
    }

    public int getNextRecordID() throws RecordStoreException {
        return 0;
    }

    public int getRecordSize(int recordId) throws RecordStoreException {
        return 0;
    }

    public byte[] getRecord(int recordId) throws RecordStoreException {
        return new byte[0];
    }

    public int addRecord(byte[] data, int offset, int numBytes)
            throws RecordStoreException {
        return 0;
    }

    public void setRecord(int recordId, byte[] newData, int offset, int numBytes)
            throws RecordStoreException {
    }

    public void deleteRecord(int recordId) throws RecordStoreException {
    }
}
