#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use unix::RawMode;
#[cfg(windows)]
pub use windows::RawMode;
