//! Overlapped I/O helpers for Windows named pipes.
//!
//! Windows named pipes opened with FILE_FLAG_OVERLAPPED require asynchronous
//! I/O operations. This module provides synchronous wrappers that internally
//! use overlapped structures with manual-reset events, allowing the relay
//! threads to block on I/O while remaining interruptible.
//!
//! The EventPool amortizes event handle creation across many I/O operations,
//! avoiding repeated CreateEvent/CloseHandle syscalls in the hot path.

use std::io;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{
    CreateEventW, ResetEvent, WaitForSingleObject, INFINITE,
};

const ERROR_IO_PENDING: u32 = 997;

pub struct EventPool {
    inner: Mutex<Vec<HANDLE>>,
}

// SAFETY: EventPool only contains a Mutex<Vec<HANDLE>>. HANDLEs are OS-level
// identifiers that are safe to send between threads. The Mutex provides synchronization.
unsafe impl Send for EventPool {}
unsafe impl Sync for EventPool {}

impl Default for EventPool {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPool {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }

    pub fn get(&self) -> io::Result<HANDLE> {
        let mut events = self.inner.lock().unwrap();
        if let Some(h) = events.pop() {
            reset_event(h);
            return Ok(h);
        }
        drop(events);

        create_manual_reset_event()
    }

    pub fn put(&self, h: HANDLE) {
        self.inner.lock().unwrap().push(h);
    }
}

impl Drop for EventPool {
    fn drop(&mut self) {
        let events = self.inner.lock().unwrap();
        for &h in events.iter() {
            unsafe {
                CloseHandle(h);
            }
        }
    }
}

/// A handle that is guaranteed to have been opened with FILE_FLAG_OVERLAPPED.
/// This type encodes the invariant at construction time, allowing async_read/async_write
/// to be safe to call.
#[derive(Debug, Clone, Copy)]
pub struct OverlappedHandle(HANDLE);

// SAFETY: Windows HANDLEs are safe to send between threads. The underlying OS handle
// can be used from any thread, and the OverlappedHandle is just a wrapper around
// a pointer-sized value that doesn't depend on thread-local state.
unsafe impl Send for OverlappedHandle {}
unsafe impl Sync for OverlappedHandle {}

impl OverlappedHandle {
    /// Creates an OverlappedHandle from a raw HANDLE.
    ///
    /// # Safety
    /// The caller must ensure `handle` is a valid handle opened with FILE_FLAG_OVERLAPPED
    /// and owned by this process.
    pub unsafe fn from_raw(handle: HANDLE) -> Self {
        Self(handle)
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

fn create_manual_reset_event() -> io::Result<HANDLE> {
    let h = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
    if h.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(h)
    }
}

fn reset_event(handle: HANDLE) {
    let result = unsafe { ResetEvent(handle) };
    debug_assert!(result != 0, "ResetEvent failed on valid handle");
}

fn check_io_pending() -> io::Result<()> {
    let err = unsafe { GetLastError() };
    if err == ERROR_IO_PENDING {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(err as i32))
    }
}

struct EventGuard<'a> {
    pool: &'a EventPool,
    handle: HANDLE,
}

impl<'a> EventGuard<'a> {
    fn new(pool: &'a EventPool) -> io::Result<Self> {
        let handle = pool.get()?;
        Ok(Self { pool, handle })
    }
}

impl<'a> Drop for EventGuard<'a> {
    fn drop(&mut self) {
        self.pool.put(self.handle);
    }
}

pub fn async_read(
    handle: OverlappedHandle,
    buf: &mut [u8],
    pool: &EventPool,
) -> io::Result<usize> {
    debug_assert!(!buf.is_empty(), "async_read called with empty buffer");

    let event_guard = EventGuard::new(pool)?;

    let mut overlapped = OVERLAPPED::default();
    overlapped.hEvent = event_guard.handle;

    let mut bytes_read: u32 = 0;
    let result = unsafe {
        ReadFile(
            handle.raw(),
            buf.as_mut_ptr().cast(),
            buf.len() as u32,
            &mut bytes_read,
            &mut overlapped,
        )
    };

    if result != 0 {
        return Ok(bytes_read as usize);
    }

    check_io_pending()?;

    let wait_result = unsafe { WaitForSingleObject(event_guard.handle, INFINITE) };
    if wait_result != WAIT_OBJECT_0 {
        return Err(io::Error::last_os_error());
    }

    let mut transferred: u32 = 0;
    let success = unsafe { GetOverlappedResult(handle.raw(), &overlapped, &mut transferred, 0) };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(transferred as usize)
}

pub fn async_write(handle: OverlappedHandle, buf: &[u8], pool: &EventPool) -> io::Result<usize> {
    let event_guard = EventGuard::new(pool)?;

    let mut overlapped = OVERLAPPED::default();
    overlapped.hEvent = event_guard.handle;

    let mut bytes_written: u32 = 0;
    let result = unsafe {
        WriteFile(
            handle.raw(),
            buf.as_ptr().cast(),
            buf.len() as u32,
            &mut bytes_written,
            &mut overlapped,
        )
    };

    if result != 0 {
        return Ok(bytes_written as usize);
    }

    check_io_pending()?;

    let wait_result = unsafe { WaitForSingleObject(event_guard.handle, INFINITE) };
    if wait_result != WAIT_OBJECT_0 {
        return Err(io::Error::last_os_error());
    }

    let mut transferred: u32 = 0;
    let success = unsafe { GetOverlappedResult(handle.raw(), &overlapped, &mut transferred, 0) };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(transferred as usize)
}
