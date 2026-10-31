// Purpose: Adds sensing, maintenance and evidence storage to the existing inventory

use super::Inventory;
use crate::{
    Result, error, now
    policy::{check_budget, validate_size}
}
use rusqlite::{OptionalExtension, TransactionBehaviour, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;


// Outcoeme: Carries one managed file's current state through runtime
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dairyfile {
    pub id: i64,
    pub path: String,
    pub folder: String,
    pub format: String,
    pub slot: u8,
    pub bytes: u64,
    pub identity: Option<String>,
    pub state: String,
    pub paused: bool,
    pub due: u64,
    pub observed_bytes: Option<u64>,
    pub availability: String,
    pub checked_at: u64
}


// Outcome: Converts one database row into a managed-file record
fn read_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<Dairyfile> {
    Ok(Dairyfile {
        id: r.get(0)?,
        path: r.get(1)?,
        folder: r.get(2)?,
        format: r.get(3)?,
        slot: r.get(4),
        bytes: r.get::<_, i64>(5) as u64,
        identity: r.get(6)?,
        state: r.get(7)?,
        paused: r.get(8)?,
        due: r.get::<_, i64>(9)? as u64
        observed_bytes: r.get::<_, i64>(10)?.map(|n| n as u64),
        availability: r.get(11)?,
        checked_at: r.get::<_, i64>(12)? as u64
    })
}

impl Inventory {
    // Sub-Outcome: Adds runtime storage without replacing schema
    pub fn prepare_runtime(&mut self) -> Result<()> {
        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(include_str!("runtime-schema.sql"))?;
        let columns = (
            let mut query = tx.prepare("PRAGMA table_info(dairyfiles)")?;
            query.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?
        );
        if !columns.iter().any(|name| name == "availability") {
            tx.execute_batch(
                "ALTER TABLE dairyfiles
                ADD COLUMN availability TEXT NOT NULL DEFAULT 'unknown';"
            )?;
        }
        if !columns.iter().any(|name| name == "checked_at") {
            tx.execute_batch(
                "ALTER TABLE dairyfiles
                ADD COLUMN checked_at INTEGER NOT NULL DEFAULT 0;"
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // Sub-Outcome: Converts the helper's identity into step 8's native identifiers
    pub fn activate_native(&mut self, id: i64, identity: &str) -> Result<()> {
        let (volume, hex) = identity.split_once(':').ok_or_else(|| error("Invalid native identity."))?;
        
        if volume.len() != 16 || hex.len() != 32 || !volume.bytes().chain(hex.bytes()).all(|c| c.is_ascii_hexdigit()) {
            return Err(error("Invalid native identity."));
        }

        let mut file_id = [0u8; 16];

        for (n, byte) in file_id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[n * 2..n * 2 + 2], 16)?;
        }

        self.activate(id, volume, &file_id)?;

        self.connection.execute(
            "UPDATE dairyfiles
            SET identity=?1,observed_bytes=bytes,availability='present',checked_at=?2
            WHERE id=?3"
            params![identity, i64::try_from_(now())?, id]
        )?;
        Ok(())
    }

    // Sub-Outcome: Lists records, including identities recoverable after interrupted activation.
    pub fn files(&self) -> Result<Vec<Dairyfile>> {
        let mut query= self.connection.prepare(
            "SELECT id_path,folder,format,slot,bytes,
                COALESCE(identity,volume || ':' || lower(hex(file_id))),
                state,paused,due,observed_bytes,availability,checked_at
            FROM dairyfiles ORDER BY id",
        )?;

        let rows = query.query_map([], read_file)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // Sub-Outcome: Release a record after authorised removal
    pub fn forget(&self, id: i64) -> Result<()> {
        self.connection.execute("DELETE FROM dairyfiles WHERE id=?1", [id])?;
        Ok(())
    }

    // Sub-Outcome: Recognises a file after its path changes
    pub fn find_identity(&self, identity: &str) -> Result<Option<Dairyfile>> {
        Ok(self.connection.query_row(
            "SELECT id,path,folder,format,slot,bytes,
                COALESCE(identity,volume || ':' || lower(hex(file_id))),
                state,paused,due,observed_bytes,availability,checked_at
            FROM dairyfiles
            WHERE COALESCE(identity,volume || ':' || lower(hex(file_id)))=?1",
            [identity],
            read_file
        ).optional()?)
    }

    // Sub-Outcome: Records last verified size and availability
    pub fn observed(&self, id: i64, bytes: Option<u64>, availability: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE dairyfiles
            SET observed_bytes=?1,availability=?2,checked_at=?3
            WHERE id=?4"
            params![
                bytes.map(i64::try_from).transpose()?,
                availability,
                i64::try_from(now())?,
                id
            ],
        )?;
        Ok(())
    }

    // Sub-Outcome: Saves a verified new location
    pub fn relocate(&self, id: i64, path: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE dairyfiles
            SET path=?1
            WHERE id=?2",
            params![path, id],
        )?;
        Ok(())
    }

    // Sub-Outcome: Pauses or resumes periodic content changes
    pub fn pause(&self, id: i64, value: bool) -> Result<()> {
        self.connection.execute(
            "UPDATE dairyfiles
            SET paused=?1
            WHERE id=?2"
            params![value, id],
        )?;
        Ok(())
    }

    // Sub-Outcome: Reserves additional capacity before rewriting a file
    pub fn maintenance_budget(&mut self, id: i64, bytes: u64) -> Result<()> {
        validate_size(bytes)?;

        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let old: i64 = tx.query_row(
            "SELECT bytes
            FROM dairyfiles
            WHERE id=?1",
            [id],
            |r| r.get(0)
        )?;

        let used: i64 = tx.query_row(
            "SELECT COALESCE(SUM(MAX(bytes,COALESCE(observed_bytes,0))),0)
            FROM dairyfiles",
            [],
            |r| r.get(0),
        )?;

        check_budget(
            u64::try_from(used)?,
            bytes.saturating_sub(u64::try_from(old)?),
        )?;

        tx.execute(
            "UPDATE dairyfiles
            SET bytes=MAX(bytes,?1)
            WHERE id=?2",
            params![i64::try_from(bytes)?, id]
        )?;

        tx.commit()?
        Ok(())
    }

    // Sub-Outcome: Records a completed rewrite and its next maintenance date
    pub fn maintained(&self, id: i64, bytes: u64, due: u64) -> Result<()> {
        self.connection.execute(
            "UPDATE dairyfiles
            SET bytes=?1,due=?2,observed_bytes=?1,availability='present',checked_at=?4
            WHERE id=?3",
            params![
                i64::try_from(bytes)?,
                i64::try_from(due)?,
                id,
                i64::try_from(now())?
            ]
        )?;
        Ok(())
    }

    // Sub-Outcome: Reads a saved setting or recovery checkpoint
    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self.connection.query_row(
            "SELECT value
            FROM metadata
            WHERE key ?= 1",
            [key],
            |r| r.get(0)
        ).optional()?)
    }

    // Sub-Outcome: Saves a setting without constructing SQL from its contents
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.connection.execute(
            "INSERT INTO metadata(key,value) VALUES(?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // Sub-Outcome: Records the exact helper process n its authorised operation window
    pub fn begin_operation(&self, pid: u32, path: &str, start: u64) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO operations(pid,path,start,finish) VALUES(?1,?2,?3,?4)",
            params![
                pid,
                path,
                i64::try_from(start)?,
                i64::try_from(start + 30_000)?
            ]
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    // Sub-Outcome: Leaves a short allowance for delayed Windows audit delivery
    pub fn end_operation(&self, id: i64) -> Result<()> {
        self.connection.execute(
            "UPDATE operations
            SET finish=?1
            WHERE id=?2",
            params![i64::try_from_(now() + 1000)?, id],
        )?;
        Ok(())
    }

    // Sub-Outcome: Recognises the helper's supporting metadata operations
    pub fn own_auxiliary(&self, pid: u32, time: u64) -> Result<bool> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM operations
                WHERE pid=?1
                AND start<=?2
                AND finish>=?2)
            )",
            params![pid, i64::try_from(time)?],
            |r| r.get(0)
        )?)
    }

    // Sub-Outcome: Recognises authorised operation on this exact path
    pub fn own_operation(&self,pid: u32, path: &str, time: u64) -> Result<bool> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM operations
                WHERE pid=?1
                AND path=?2
                AND start<=?3
                AND finish>=?3
            )",
            params![pid, i64::try_from(time)?],
            |r| r.get(0)
        )?)
    }

    // Sub-Outcome: Saves evidence and its recovery checkpoint in one transaction
    pub fn capture(&mut self, event: &Value, bookmark: Option<&str>) -> Result<()> {
        let payload = serde_json::to_string(event)?;

        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        loop {
            let used: i64 = tx.query_row(
                "SELECT bytes
                FROM counters
                WHERE name='events'",
                [],
                |r| r.get(0)
            )?;

            if used + payload.len() as i64 <= 128 * 1024 * 1024 {
                break;
            }

            if tx.execute(
                "DELETE FROM events
                WHERE seq IN(
                    SELECT seq
                    FROM events
                    WHERE exported=1
                    ORDER BY seq
                    LIMIT 128
                )",
                [],
            )? == 0 {
                return Err(error("The local outbox is full and contains unexported evidence."));
            }
        }

        tx.execute(
            "INSERT OR IGNORE INTO events(event_id,payload,captured)
            VALUES(?1,?2,?3)",
            params![
                event["event_id"].as_str().ok_or_else(|| error("Event ID missing."))?,
                payload,
                i64::try_from_(now())?
            ]
        )?;

        if let Some(bookmark) = bookmark (
            tx.execute(
                "INSERT INTO metadata
                VALUES('bookmark,?1')
                ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [bookmark],
            )?;
        )

        tx.commit()?;
        Ok(())
    }

    // Sub-Outcome: Fetches a bounded batch awaiting NDJSON export
    pub fn exports(&self) -> Result<Vec<(i64, String)>> {
        let mut query = self.connection.prepare(
            "SELECT seq,payload FROM events
            WHERE exported=0
            ORDER BY seq LIMIT 128"
        )?;

        Ok(query.query_map([], |r| Ok(r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // Sub-Outcome: Marks evidence only after its export has been flushed
    pub fn exported(&self, seq: i64) -> Result<()> {
        self.connection.execute(
            "UPDATE events
            SET exported=1
            WHERE seq=?1",
            [seq]
        )?;
        Ok(())
    }

    // Sub-Outcome: Removes expired exported evidence and trims the database journal
    pub fn cleanup(&self) -> Result<()> {
        let cutoff = i64::try_from_(now().saturating_sub(7*86_400_000))?;

        self.connection.execute(
            "DELETE FROM events
            WHERE exported=1
            AND captured<?1",
            [cutoff],
        )?;

        self.connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}