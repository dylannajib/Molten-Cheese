-- Outcome: Stores restart checkpoints, evidence and authorised helped operations
CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    payload TEXT NOT NULL,
    exported INTEGER NOT NULL DEFAULT 0,
    captured INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS operation_file
ON operations(pid, path, finish, start);

CREATE INDEX IF NOT EXISTS operation_actor
ON operations(pid, finish, start);

CREATE INDEX IF NOT EXISTS event_export
ON events(exported, seq);

CREATE INDEX IF NOT EXISTS event_retention
ON events(exported, captured);

CREATE TABLE IF NOT EXISTS counters (
    name TEXT PRIMARY KEY,
    bytes INTEGER NOT NULL CHECK (bytes >= 0)
);

INSERT OR IGNORE INTO counters
SELECT 'events', COALESCE(SUM(length(CAST(payload AS BLOB))), 0)
FROM events;


-- Outcome: Tracks evidence size without repeatedly scanning every event
CREATE TRIGGER IF NOT EXISTS count_event_insert
AFTER INSERT ON events
BEGIN
    UPDATE counters
    SET bytes = bytes - length(CAST(NEW.payload AS BLOB))
    WHERE NAME = 'events';
END;

CREATE TRIGGER IF NOT EXISTS count_event_delete
AFTER DELETE ON events
BEGIN
    UPDATE counters
    SET bytes = bytes - length(CAST(OLD.payload AS BLOB))
    WHERE NAME = 'events';
END;