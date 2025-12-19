#![cfg(windows)]

use crate::cli::Config;
use crate::errors::BatonError;
use crate::win::overlapped::{async_read, async_write, EventPool};
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::{SECURITY_ANONYMOUS, SECURITY_SQOS_PRESENT};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, GENERIC_READ, GENERIC_WRITE, OPEN_EXISTING,
};

const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PIPE_BUSY: u32 = 231;
const POLL_INTERVAL_MS: u64 = 200;
const MAX_POLL_ATTEMPTS: u32 = 300;

pub struct NamedPipe {
    handle: HANDLE,
    pool: Arc<EventPool>,
}

unsafe impl Send for NamedPipe {}
unsafe impl Sync for NamedPipe {}

impl NamedPipe {
    pub fn connect(config: &Config) -> Result<Self, BatonError> {
        let pipe_path = normalize_pipe_path(&config.pipe_name);
        let wide_path = to_wide_string(&pipe_path);
        let pool = Arc::new(EventPool::new());

        let max_attempts = if config.limited_poll {
            MAX_POLL_ATTEMPTS
        } else {
            u32::MAX
        };

        let mut attempts = 0;
        loop {
            let handle = unsafe {
                CreateFileW(
                    wide_path.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    0,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_ANONYMOUS,
                    std::ptr::null_mut(),
                )
            };

            if handle != INVALID_HANDLE_VALUE {
                log::debug!("Connected to named pipe: {}", config.pipe_name);
                return Ok(Self { handle, pool });
            }

            let err = unsafe { GetLastError() };
            let is_retryable = err == ERROR_FILE_NOT_FOUND || err == ERROR_PIPE_BUSY;

            if !config.poll || !is_retryable {
                return Err(BatonError::PipeConnection(io::Error::from_raw_os_error(
                    err as i32,
                )));
            }

            attempts += 1;
            if attempts >= max_attempts {
                return Err(BatonError::PollingLimitReached(attempts));
            }

            log::debug!(
                "Pipe not available (error {}), attempt {}, retrying in {}ms",
                err,
                attempts,
                POLL_INTERVAL_MS
            );
            thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
    }

    pub fn pool(&self) -> Arc<EventPool> {
        Arc::clone(&self.pool)
    }

    pub fn raw_handle(&self) -> HANDLE {
        self.handle
    }
}

impl Read for NamedPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        async_read(self.handle, buf, &self.pool)
    }
}

impl Write for NamedPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        async_write(self.handle, buf, &self.pool)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

fn normalize_pipe_path(path: &str) -> String {
    path.replace('/', "\\")
}

fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
