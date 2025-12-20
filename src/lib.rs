#![deny(warnings)]
#![deny(clippy::all)]

pub mod cli;
pub mod errors;
pub mod logging;
pub mod relay;

#[cfg(windows)]
pub mod win;
