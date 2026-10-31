// Purpose: Connects storage modules and give them one shared error format

pub mod content;     // Connects later content generation
pub mod inventory;     // Connects durable dairyfile records
pub mod platform;     // Connects later Windows operations
mod policy;     // Keeps storage checks inside backend
pub mod runtime;     // Connects later background coordination
pub use policy::{MAX_DAIRYFILE_BYTES, MAX_FILE_BYTES, MIN_FILE_BYTES};     // Shares agreed size limits

// Outcome: Lets operations return either their result or explanatory error
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;     // Shares one error format

// Outcome: Creates one consistent error type for backend operations
pub fn error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {     // Accepts readable error message
    std::io::Error::other(message.into()).into()     // Wraps message in shared error format
}     // Ends error conversion


// Outcome: Returns current time in milliseconds
pub fn now() -> u64 {     // Provides shared timestamp format
    std::time::SystemTime::now()     // Reads current system time
        .duration_since(std::time::UNIX_EPOCH)     // Measures time since January 1970
        .unwrap_or_default()     // Uses zero if clock predates that point
        .as_millis() as u64     // Converts elapsed time to milliseconds
}     // Ends timestamp conversion
