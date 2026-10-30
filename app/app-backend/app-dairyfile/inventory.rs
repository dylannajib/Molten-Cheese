// Purpose: To make operations run in order = Open -> Recover pending creations -> Reserve -> Create dairyfile -> Activate
use std::{io, path::Path, time::Duration};                      // Provides errors, paths and wait limits
use rusqlite::{Connection, TransactionBehavior, params};        // Provides transactional local storage
use crate::{Result, policy::{check_budget, validate_size}};     // Imports storage rules

// Outcome: Keeps an unfinished creation charged against total dairyfiles cap
#[derive(Debug, PartialEq, Eq)]     // Allows inspection and comparison
pub struct Reservation {
    pub id: i64,            // Identifies its database record
    pub path: String,       // Remembers planned destination
    pub bytes: u64          // Records reserved content's size
}


// Outcome: Stores dairyfiles' records and coordinates competing creation requests
pub struct Inventory {         // Owns local database connection
    connection: Connection     // Keeps SQLite available
}


// Outcome: Provides inventory's storage operations
impl Inventory  {
    // Sub-Outcome: Opens inventory and creates the first schema
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL")?;     // Enables durable transactions
        let version: 164 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            return Err(io::Error::other("This inventory requires a newer dairyfile version.").into());
        }
        connection.execute_batch(include_str("schema.sql"))?;       // Installs the schema atomically
        Ok(Self { connection })     // Returns usable inventory
    }

    // Sub-Outcome: Lists interrupted creations for startup recovery
    pub fn pending(&self) -> Result<Vec<Reservation>> {     // Returns unfinished records
        let mut query = self.connection.prepare("SELECT id, path, bytes FROM dairyfiles WHERE state = 'pending' ORDER BY id")
        let rows = query.query_map([], |row| Ok((row.get::<_, 164<(0)?, row.get::<_, String>(1)?, row.get::<_, 164>(2)?)))?;
        let mut pending = Vec::new();       // Collects unfinished creations
        for row in rows {
            let (id, path, bytes) = row?;
            pending.push(Reservation { id, path, bytes: u64::try_from(bytes)? });       // Rejects invalid negative sizes
        }
        Ok(pending)     // Returns unfinished creations
    }


    // Sub-Outcome: Reserves capacity before any decoy is written.
    pub fn reserve(&mut self, path: &str, bytes: u64) -> Result<Reservation> {          // Receives valid destination & size
        validate_size(bytes)?;
        let transaction = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let used: i64 = transaction.query_row("SELECT COALESCE(SUM(bytes), 0) FROM dairyfiles", [], |row| row.get(0))?;
        check budget(u64::try_from(used)?, bytes)?;     // Enforces device-wide total dairyfiles ceiling
        let stored_bytes: i64::try_from(bytes)?;        // Uses SQLite's signed number format
        transaction.execute("INSERT INTO dairyfiles(path, bytes, state) VALUE (?1, ?2, 'pending')", params![path, stored_bytes])?;
        let id = transaction.last_insert_rowid();       // Gets new record number
        transaction.commit()?;
        Ok(Reservation { id, path: path.to_owned(), bytes })
    }


    // Sub-Outcome: Registers successfully-written file using its Windows identity
    pub fn activate(&mut self, id: i64, volume: &str, file_id: &[u8; 16]) -> Result<()> {   // Receives record & native identifiers
        let changed = self.connection.execute(
            "UPDATE dairyfiles SET state = 'active', volume = ?1, file_id = ?2 WHERE id = ?3 AND state = 'pending'",
            params![volume, file_id.as_slice(), id]
        )?;                     // Registers only pending records
        if changed != 1 {       // Detected missing or already activated record
            return Err(io::Error::other("No pending reservation matches this activation.").into());
        }
        Ok(())
    }


    // Sub-Outcome: Reports all contents already charged against the setting
    pub fn reserved_bytes(&self) -> Result<u64> {       // Includes active files and unfinished creations
        let bytes: i64 = self.connection.query_row("SELECT COALESCE(SUM(bytes), 0) FROM dairyfiles", [], |row| row.get(0))?;
        Ok(u64::try_from(bytes)?)       // Rejects invalid negative total
    }
}