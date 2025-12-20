use crate::cli::Config;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

const BUFFER_SIZE: usize = 32768;

pub struct RelayState {
    pub stdin_done: AtomicBool,
    pub pipe_done: AtomicBool,
}

impl RelayState {
    pub fn new() -> Self {
        Self {
            stdin_done: AtomicBool::new(false),
            pipe_done: AtomicBool::new(false),
        }
    }
}

impl Default for RelayState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_relay<R, W>(
    mut pipe_reader: R,
    mut pipe_writer: W,
    config: &Config,
) -> io::Result<()>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    let state = Arc::new(RelayState::new());
    let state_clone = Arc::clone(&state);

    let send_zero = config.send_zero;
    let exit_on_stdin_eof = config.exit_on_stdin_eof;
    let exit_on_pipe_eof = config.exit_on_pipe_eof;

    let stdin_thread = thread::spawn(move || {
        stdin_to_pipe(&mut pipe_writer, send_zero, exit_on_stdin_eof, &state_clone)
    });

    let result = pipe_to_stdout(&mut pipe_reader, exit_on_pipe_eof, &state);

    if !exit_on_pipe_eof {
        let _ = stdin_thread.join();
    }

    result
}

fn stdin_to_pipe<W: Write>(
    pipe: &mut W,
    send_zero: bool,
    exit_immediately: bool,
    state: &RelayState,
) -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        if state.pipe_done.load(Ordering::SeqCst) {
            log::debug!("Pipe closed, stopping stdin reader");
            break;
        }

        match stdin.read(&mut buffer) {
            Ok(0) => {
                log::debug!("EOF on stdin");
                state.stdin_done.store(true, Ordering::SeqCst);

                if send_zero {
                    log::debug!("Sending 0-byte message to pipe");
                    if let Err(e) = pipe.write(&[]) {
                        log::warn!("Failed to send 0-byte message: {}", e);
                    }
                }

                if exit_immediately {
                    log::debug!("Exiting immediately on stdin EOF (-ei)");
                    std::process::exit(0);
                }
                break;
            }
            Ok(n) => {
                log::debug!("Read {} bytes from stdin", n);
                if let Err(e) = pipe.write_all(&buffer[..n]) {
                    if is_broken_pipe(&e) {
                        log::debug!("Pipe broken while writing");
                        state.pipe_done.store(true, Ordering::SeqCst);
                        break;
                    }
                    return Err(e);
                }
            }
            Err(e) => {
                log::warn!("Error reading stdin: {}", e);
                state.stdin_done.store(true, Ordering::SeqCst);
                break;
            }
        }
    }

    Ok(())
}

fn pipe_to_stdout<R: Read>(
    pipe: &mut R,
    exit_immediately: bool,
    state: &RelayState,
) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    let mut buffer = vec![0u8; BUFFER_SIZE];

    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => {
                log::debug!("EOF on pipe (0 bytes read)");
                state.pipe_done.store(true, Ordering::SeqCst);

                if exit_immediately {
                    log::debug!("Exiting immediately on pipe EOF (-ep)");
                    std::process::exit(0);
                }
                break;
            }
            Ok(n) => {
                log::debug!("Read {} bytes from pipe", n);
                stdout.write_all(&buffer[..n])?;
                stdout.flush()?;
            }
            Err(e) => {
                if is_broken_pipe(&e) {
                    log::debug!("Pipe broken");
                    state.pipe_done.store(true, Ordering::SeqCst);

                    if exit_immediately {
                        log::debug!("Exiting immediately on pipe EOF (-ep)");
                        std::process::exit(0);
                    }
                    break;
                }
                return Err(e);
            }
        }
    }

    Ok(())
}

fn is_broken_pipe(e: &io::Error) -> bool {
    matches!(e.kind(), io::ErrorKind::BrokenPipe)
        || e.raw_os_error() == Some(109) // ERROR_BROKEN_PIPE
        || e.raw_os_error() == Some(233) // ERROR_PIPE_NOT_CONNECTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, ErrorKind};

    #[test]
    fn test_relay_state_initial() {
        let state = RelayState::new();
        assert!(!state.stdin_done.load(Ordering::SeqCst));
        assert!(!state.pipe_done.load(Ordering::SeqCst));
    }

    #[test]
    fn test_relay_state_set() {
        let state = RelayState::new();
        state.stdin_done.store(true, Ordering::SeqCst);
        state.pipe_done.store(true, Ordering::SeqCst);
        assert!(state.stdin_done.load(Ordering::SeqCst));
        assert!(state.pipe_done.load(Ordering::SeqCst));
    }

    #[test]
    fn test_is_broken_pipe_error_kind() {
        let e = io::Error::new(ErrorKind::BrokenPipe, "broken pipe");
        assert!(is_broken_pipe(&e));
    }

    #[test]
    fn test_is_broken_pipe_windows_error_109() {
        let e = io::Error::from_raw_os_error(109);
        assert!(is_broken_pipe(&e));
    }

    #[test]
    fn test_is_broken_pipe_windows_error_233() {
        let e = io::Error::from_raw_os_error(233);
        assert!(is_broken_pipe(&e));
    }

    #[test]
    fn test_is_broken_pipe_other_error() {
        let e = io::Error::new(ErrorKind::NotFound, "not found");
        assert!(!is_broken_pipe(&e));
    }

    #[test]
    fn test_buffer_size_constant() {
        assert_eq!(BUFFER_SIZE, 32768);
    }

    struct MockWriter {
        data: Vec<u8>,
        write_error: Option<ErrorKind>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                data: Vec::new(),
                write_error: None,
            }
        }

        fn with_error(kind: ErrorKind) -> Self {
            Self {
                data: Vec::new(),
                write_error: Some(kind),
            }
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if let Some(kind) = self.write_error {
                return Err(io::Error::new(kind, "mock error"));
            }
            self.data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_mock_writer_captures_data() {
        let mut writer = MockWriter::new();
        writer.write_all(b"hello").unwrap();
        assert_eq!(writer.data, b"hello");
    }

    #[test]
    fn test_mock_writer_error() {
        let mut writer = MockWriter::with_error(ErrorKind::BrokenPipe);
        let result = writer.write(b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_cursor_as_mock_reader() {
        let data = b"test data";
        let mut reader = Cursor::new(data.to_vec());
        let mut buf = [0u8; 9];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 9);
        assert_eq!(&buf, data);
    }
}
