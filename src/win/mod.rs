#![cfg(windows)]

pub mod console;
pub mod overlapped;
pub mod pipe;

pub use console::hide_console_window;
pub use pipe::NamedPipe;
