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
