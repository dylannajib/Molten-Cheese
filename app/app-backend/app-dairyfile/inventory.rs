// Purpose: To make operations run in order = Open -> Recover pending creations -> Reserve -> Create dairyfile -> Activate
use std::{io, path::Path, time::Duration};                      // Provides errors, paths and wait limits
use rusqlite::{Connection, TransactionBehavior, params};        // Provides transactional local storage
use crate::{Result, policy::{check_budget, validate_size}};     // Imports storage rules

// Outcome: Keeps an unfinished creation charged against total dairyfiles cap
#[derive(Debug, PartialEq, Eq)]     // Allows inspection and comparison
pub struct Reservation {     // Groups unfinished creation details
    pub id: i64,            // Identifies its database record
    pub path: String,       // Remembers planned destination
    pub bytes: u64          // Records reserved content's size
}     // Ends reservation details


// Outcome: Stores dairyfiles' records and coordinates competing creation requests
pub struct Inventory {         // Owns local database connection
    connection: Connection     // Keeps SQLite available
}     // Ends inventory details


// Outcome: Provides inventory's storage operations
impl Inventory {     // Groups inventory operations
    // Sub-Outcome: Opens inventory and creates the first schema
    pub fn open(path: &Path) -> Result<Self> {     // Receives inventory database path
        let connection = Connection::open(path)?;     // Opens or creates local inventory
        connection.busy_timeout(Duration::from_secs(5))?;     // Limits waits for competing writes
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL")?;     // Enables durable transactions
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;     // Reads saved schema version
        if version > 1 {     // Detects newer inventory format
            return Err(io::Error::other("This inventory requires a newer dairyfile version.").into());     // Prevents incompatible access
        }     // Ends version check
        connection.execute_batch(include_str!("schema.sql"))?;       // Installs the schema atomically
        Ok(Self { connection })     // Returns usable inventory
    }     // Ends inventory opening

    // Sub-Outcome: Lists interrupted creations for startup recovery
    pub fn pending(&self) -> Result<Vec<Reservation>> {     // Returns unfinished records
        let mut query = self.connection.prepare("SELECT id, path, bytes FROM dairyfiles WHERE state = 'pending' ORDER BY id")?;     // Prepares unfinished-record lookup
        let rows = query.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))?;     // Reads record number, path & reserved size
        let mut pending = Vec::new();       // Collects unfinished creations
        for row in rows {     // Visits each unfinished creation
            let (id, path, bytes) = row?;     // Stops if record could not be read
            pending.push(Reservation { id, path, bytes: u64::try_from(bytes)? });       // Rejects invalid negative sizes
        }     // Ends unfinished-record collection
        Ok(pending)     // Returns unfinished creations
    }     // Ends recovery lookup


    // Sub-Outcome: Reserves capacity before any decoy is written.
    pub fn reserve(&mut self, path: &str, folder: &str, format: &str, slot: u8, bytes: u64, due: u64) -> Result<Reservation> {     // Receives required placement details
        validate_size(bytes)?;     // Rejects unsupported generated size
        let stored_bytes = i64::try_from(bytes)?;     // Uses SQLite's signed size format
        let stored_due = i64::try_from(due)?;     // Rejects unsupported maintenance timestamp
        let transaction = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;     // Reserves sole database write access
        let used: i64 = transaction.query_row("SELECT COALESCE(SUM(bytes), 0) FROM dairyfiles", [], |row| row.get(0))?;     // Includes active files & pending reservations
        check_budget(u64::try_from(used)?, bytes)?;     // Enforces device-wide total dairyfiles ceiling
        transaction.execute("INSERT INTO dairyfiles(path, folder, format, slot, bytes, state, due) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)", params![path, folder, format, slot, stored_bytes, stored_due])?;     // Saves complete unfinished creation
        let id = transaction.last_insert_rowid();       // Gets new record number
        transaction.commit()?;     // Makes reservation durable before file creation
        Ok(Reservation { id, path: path.to_owned(), bytes })     // Returns reserved record & destination
    }     // Ends storage reservation


    // Sub-Outcome: Registers successfully-written file using its Windows identity
    pub fn activate(&mut self, id: i64, volume: &str, file_id: &[u8; 16]) -> Result<()> {   // Receives record & native identifiers
        let changed = self.connection.execute(     // Updates matching unfinished creation
            "UPDATE dairyfiles SET state = 'active', volume = ?1, file_id = ?2 WHERE id = ?3 AND state = 'pending'",     // Keeps completed identity tied to its reservation
            params![volume, file_id.as_slice(), id]     // Passes identifiers without building SQL text
        )?;                     // Registers only pending records
        if changed != 1 {       // Detected missing or already activated record
            return Err(io::Error::other("No pending reservation matches this activation.").into());     // Explains rejected activation
        }     // Ends matching-record check
        Ok(())     // Confirms completed registration
    }     // Ends file activation


    // Sub-Outcome: Reports all contents already charged against the ceiling
    pub fn reserved_bytes(&self) -> Result<u64> {       // Includes active files and unfinished creations
        let bytes: i64 = self.connection.query_row("SELECT COALESCE(SUM(bytes), 0) FROM dairyfiles", [], |row| row.get(0))?;     // Totals active & unfinished content sizes
        Ok(u64::try_from(bytes)?)       // Rejects invalid negative total
    }     // Ends reserved-size lookup
}     // Ends inventory operations

#[path="inventory-runtime.rs"]
mod runtime_storage;
pub use runtime_storage::Dairyfile;