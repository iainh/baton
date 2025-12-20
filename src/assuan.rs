use baton::cli::Config;
use baton::errors::BatonError;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

const NONCE_SIZE: usize = 16;
const POLL_INTERVAL_MS: u64 = 200;
const MAX_POLL_ATTEMPTS: u32 = 300;

pub fn connect_assuan(config: &Config) -> Result<TcpStream, BatonError> {
    let (port, nonce) = parse_assuan_file(&config.pipe_name)?;

    log::debug!("Assuan port: {}, nonce length: {}", port, nonce.len());

    let addr = format!("127.0.0.1:{}", port);
    let mut stream = connect_with_retry(&addr, config)?;

    use std::io::Write;
    stream
        .write_all(&nonce)
        .map_err(BatonError::AssuanConnection)?;

    log::debug!("Assuan nonce sent successfully");

    Ok(stream)
}

fn parse_assuan_file(path: &str) -> Result<(u16, Vec<u8>), BatonError> {
    let file = File::open(path).map_err(|e| BatonError::AssuanParse(format!("cannot open file: {}", e)))?;
    let mut reader = BufReader::new(file);

    let mut port_line = String::new();
    reader
        .read_line(&mut port_line)
        .map_err(|e| BatonError::AssuanParse(format!("cannot read port line: {}", e)))?;

    let port_str = port_line.trim_end_matches(|c| c == '\r' || c == '\n');
    let port: u16 = port_str
        .parse()
        .map_err(|e| BatonError::AssuanParse(format!("invalid port number '{}': {}", port_str, e)))?;

    let mut nonce = vec![0u8; NONCE_SIZE];
    reader
        .read_exact(&mut nonce)
        .map_err(|e| BatonError::AssuanParse(format!("cannot read nonce (need {} bytes): {}", NONCE_SIZE, e)))?;

    Ok((port, nonce))
}

fn connect_with_retry(addr: &str, config: &Config) -> Result<TcpStream, BatonError> {
    let max_attempts = if config.limited_poll {
        MAX_POLL_ATTEMPTS
    } else {
        u32::MAX
    };

    let mut attempts = 0;
    loop {
        match TcpStream::connect(addr) {
            Ok(stream) => {
                log::debug!("Connected to Assuan TCP socket at {}", addr);
                return Ok(stream);
            }
            Err(e) => {
                if !config.poll {
                    return Err(BatonError::AssuanConnection(e));
                }

                attempts += 1;
                if attempts >= max_attempts {
                    return Err(BatonError::PollingLimitReached(attempts));
                }

                log::debug!(
                    "Connection attempt {} failed: {}, retrying in {}ms",
                    attempts,
                    e,
                    POLL_INTERVAL_MS
                );
                thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_assuan_file(port: u16, nonce: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", port).unwrap();
        file.write_all(nonce).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_parse_assuan_file_valid() {
        let nonce = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let file = create_test_assuan_file(8080, &nonce);

        let (port, parsed_nonce) = parse_assuan_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(port, 8080);
        assert_eq!(parsed_nonce, nonce);
    }

    #[test]
    fn test_parse_assuan_file_invalid_port() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "not_a_number").unwrap();
        file.flush().unwrap();

        let result = parse_assuan_file(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_assuan_file_short_nonce() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "8080").unwrap();
        file.write_all(&[1, 2, 3]).unwrap(); // Only 3 bytes, need 16
        file.flush().unwrap();

        let result = parse_assuan_file(file.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
