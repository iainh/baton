mod assuan;
mod cli;
mod errors;
mod logging;
mod relay;

#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() {
    if let Err(e) = real_main() {
        eprintln!("baton error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("baton is Windows-only (target a Windows triple to run).");
    std::process::exit(1);
}

#[cfg(windows)]
fn real_main() -> anyhow::Result<()> {
    use crate::win::{hide_console_window, NamedPipe};

    let config = cli::parse();
    logging::init_logging(config.verbose);

    if config.bg {
        hide_console_window();
    }

    log::debug!("Config: {:?}", config);

    if config.assuan {
        let stream = assuan::connect_assuan(&config)?;
        let reader = stream.try_clone()?;
        let writer = stream;
        relay::run_relay(reader, writer, &config)?;
    } else {
        let pipe = NamedPipe::connect(&config)?;
        let pool = pipe.pool();
        let handle = pipe.raw_handle();

        let reader = PipeReader { handle, pool: pool.clone() };
        let writer = PipeWriter { handle, pool };

        relay::run_relay(reader, writer, &config)?;
    }

    Ok(())
}

#[cfg(windows)]
struct PipeReader {
    handle: windows_sys::Win32::Foundation::HANDLE,
    pool: std::sync::Arc<win::overlapped::EventPool>,
}

#[cfg(windows)]
impl std::io::Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        win::overlapped::async_read(self.handle, buf, &self.pool)
    }
}

#[cfg(windows)]
struct PipeWriter {
    handle: windows_sys::Win32::Foundation::HANDLE,
    pool: std::sync::Arc<win::overlapped::EventPool>,
}

#[cfg(windows)]
impl std::io::Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        win::overlapped::async_write(self.handle, buf, &self.pool)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(windows)]
unsafe impl Send for PipeReader {}

#[cfg(windows)]
unsafe impl Send for PipeWriter {}
