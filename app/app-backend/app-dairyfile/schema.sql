-- Outcome: Stores creation progress so it survives a restart
BEGIN IMMEDIATE; -- Prevents competing schema changes
CREATE TABLE IF NOT EXISTS dairyfiles (  -- Stores active dairyfiles & unfinished creations
    id INTEGER PRIMARY KEY AUTOINCREMENT,    -- Keeps record numbers from being reused
    path TEXT NOT NULL UNIQUE CHECK(length(path) > 0), -- Records non-empty destination
    folder TEXT NOT NULL CHECK(length(folder) > 0), -- Records destination folder
    format TEXT NOT NULL CHECK(format IN ('docx', 'xlsx', 'pptx', 'pdf', 'txt', 'csv', 'jpg', 'png', 'zip')), -- Allows agreed formats
    slot INTEGER NOT NULL CHECK(slot BETWEEN 0 AND 2), -- Records first, middle or last position
    bytes INTEGER NOT NULL CHECK(bytes BETWEEN 4096 AND 32768), -- Stores reserved content size
    identity TEXT UNIQUE, -- Keeps optional combined identity unique
    state TEXT NOT NULL CHECK(state IN ('pending', 'active')), -- Distinguishes unfinished & completed creations
    paused INTEGER NOT NULL DEFAULT 0 CHECK(paused IN (0, 1)), -- Records whether maintenance is paused
    due INTEGER NOT NULL CHECK(due >= 0), -- Records next maintenance timestamp
    observed_bytes INTEGER CHECK(observed_bytes >= 0), -- Stores last measured content size
    
    volume TEXT,    -- Stores Windows volume identity
    file_id BLOB,   -- Stores Windows file identity
    UNIQUE(volume, file_id),    -- Prevents exact dairyfile duplicate
    UNIQUE(folder, slot), -- Prevents duplicate positions in one folder
    CHECK( -- Requires identity fields to match creation state
        (state = 'pending' AND volume IS NULL AND file_id IS NULL) -- Allows unfinished dairyfile
        OR (state = 'active' AND length(volume) > 0 AND volume IS NOT NULL AND -- Requires active volume identity
        length(file_id) = 16 AND file_id IS NOT NULL)   -- Requires complete active identity
    ) -- Ends creation-state validation
); -- Finishes dairyfiles table

PRAGMA user_version = 1; -- Records inventory schema version
COMMIT;     -- Makes schema.sql durable
