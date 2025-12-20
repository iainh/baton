#![cfg(windows)]

pub mod console;
pub mod overlapped;
pub mod pipe;
pub mod pipes_enum;

pub use console::hide_console_window;
pub use pipe::NamedPipe;
pub use pipes_enum::{enumerate_pipes, filter_pipes, EnumeratedPipe};
