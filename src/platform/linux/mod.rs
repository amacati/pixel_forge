//! Linux (X11) backend.
//!
//! Window and monitor lookup and capture go through the X server over an XCB connection.

mod capture;
mod error;
mod monitor;
mod window;

pub use capture::Capture;
pub use monitor::{Monitor, enumerate_monitors, primary_monitor};
pub use window::{Window, enumerate_windows, foreground_window};
