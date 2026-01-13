pub mod constants;
pub mod lib;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use unix::read_key;
#[cfg(windows)]
pub use windows::read_key;
