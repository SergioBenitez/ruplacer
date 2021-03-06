extern crate colored;
extern crate difference;
extern crate ignore;
extern crate inflector;
extern crate regex;
mod errors;
pub use errors::Error;
mod directory_patcher;
mod file_patcher;
mod line_patcher;
pub mod query;
mod stats;
pub use directory_patcher::DirectoryPatcher;
pub use stats::Stats;
