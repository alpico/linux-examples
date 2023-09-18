//! A simple multi-threaded pool for executing number-chrunching workloads.

/// Relying on crossbeam to make our life easier.
use crossbeam::channel;

mod options;
pub use options::PoolOptions;

mod sender;
pub use sender::Sender;

mod pool;
pub use pool::Pool;
