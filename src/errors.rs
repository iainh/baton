use thiserror::Error;

#[derive(Error, Debug)]
pub enum BatonError {
    #[error("Failed to connect to named pipe: {0}")]
    PipeConnection(#[source] std::io::Error),

    #[error("Polling limit reached after {0} attempts")]
    PollingLimitReached(u32),

    #[error("Failed to parse Assuan socket file: {0}")]
    AssuanParse(String),

    #[error("Failed to connect to Assuan TCP socket: {0}")]
    AssuanConnection(#[source] std::io::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
