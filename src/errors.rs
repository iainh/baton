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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_pipe_connection_error_display() {
        let err = BatonError::PipeConnection(io::Error::from_raw_os_error(2));
        let msg = format!("{}", err);
        assert!(msg.contains("Failed to connect to named pipe"));
    }

    #[test]
    fn test_polling_limit_error_display() {
        let err = BatonError::PollingLimitReached(300);
        let msg = format!("{}", err);
        assert!(msg.contains("300"));
        assert!(msg.contains("Polling limit reached"));
    }

    #[test]
    fn test_assuan_parse_error_display() {
        let err = BatonError::AssuanParse("invalid port".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid port"));
        assert!(msg.contains("Assuan socket file"));
    }

    #[test]
    fn test_assuan_connection_error_display() {
        let err = BatonError::AssuanConnection(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "connection refused",
        ));
        let msg = format!("{}", err);
        assert!(msg.contains("Assuan TCP socket"));
    }

    #[test]
    fn test_io_error_from_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err: BatonError = io_err.into();
        match err {
            BatonError::Io(_) => (),
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_error_debug_impl() {
        let err = BatonError::PollingLimitReached(100);
        let debug = format!("{:?}", err);
        assert!(debug.contains("PollingLimitReached"));
        assert!(debug.contains("100"));
    }
}
