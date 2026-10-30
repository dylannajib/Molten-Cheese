// Purpose: Connects storage modules and give them one shared error format

mod policy; // Load storage rules
mod inventory; // Load file records

pub use inventory::{Inventory, Reservation}; // Exposes working inventory
pub use policy::{MAX_DAIRYFILE_BYTES, MAX_FILE_BYTES, MIN_FILE_BYTES};

// Outcome: Lets operations return either their result or explanatory error
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>; // Shares one error format