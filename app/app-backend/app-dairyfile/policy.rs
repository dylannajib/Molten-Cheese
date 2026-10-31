// Purpose: Enforces agreed dairyfile sizes and total content ceiling.

use std::io;                    // Provides readable input errors
use crate::Result;     // Uses shared error format

pub const MIN_FILE_BYTES: u64 = 4 * 1024;           // Sets smallest dairyfile to 4 KiB
pub const MAX_FILE_BYTES: u64 = 32 * 1024;          // Sets max dairyfile to 32 KiB
pub const MAX_DAIRYFILE_BYTES: u64 = 200_000_000;   // Caps total dairyfiles at 200 MB


// Outcome: Size limit validation before reserving disk space
pub(crate) fn validate_size(bytes: u64) -> Result<()> {             // Checks one planned dairyfile
    if !(MIN_FILE_BYTES..=MAX_FILE_BYTES).contains(&bytes) {        // Rejects sizes outside our range
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Dairyfiles must be 4-32 KiB.").into());     // Explains rejected size
    }     // Ends size rejection
    Ok(())      // "Otherwise, accepts file size"
}     // Ends single-file size check


// Outcome: Refuse additions beyond MAX_DAIRYFILE_BYTES ceiling
pub(crate) fn check_budget(used: u64, extra: u64) -> Result<()> {        // Checks existing & proposed content
    let total = used.checked_add(extra).ok_or_else(|| io::Error::other("Dairyfile storage overflowed."))?;     // Rejects arithmetic overflow
    if total > MAX_DAIRYFILE_BYTES {     // Compares total with shared ceiling
        return Err(io::Error::new(io::ErrorKind::StorageFull, "200 MB dairyfile ceiling would be exceeded.").into());     // Explains rejected reservation
    }     // Ends ceiling rejection
    Ok(())     // Accepts proposed total
}     // Ends combined storage check
