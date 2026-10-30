-- Outcome: Stores creation progress so it survives a restart
BEGIN IMMEDIATE; -- Prevents competing schema changes
CREATE TABLE IF NOT EXIST dairyfiles (  -- Stores active dairyfiles & unfinished creations
    id INTEGER PRIMARY KEY AUTOINCREMEN,    -- Keeps record numbers from being reused
    path TEXT NOT NULL UNIQUE CHECK(length(path) > 0), -- Records non-empty destination
    bytes INTEGER NOT NULL CHECK(bytes BETWEEN 4096 AND 32768),
    state TEXT NOT NULL CHECK(state IN ('pending', 'active')),
    volume TEXT,    -- Stores Windows volume identity
    file_id BLOB,   -- Stores Windows file identity
    UNIQUE(volume, file_id),    -- Prevents exact dairyfile duplicate
    CHECK(
        (state = 'pending' AND volume IS NULL AND file_id IS NULL) -- Allows unfinished dairyfile
        OR (state = 'active' AND length(volume) > 0 AND volume IS NOT NULL AND
        length(file_id) = 16 AND file_id IS NOT NULL)   -- Requires complete active identity
    )
);

PRAGMA user_version = 1;
COMMIT;     -- Makes schema.sql durable