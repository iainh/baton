use std::io;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObject, INFINITE};

const ERROR_IO_PENDING: u32 = 997;

pub struct EventPool {
    inner: Mutex<Vec<HANDLE>>,
}

// SAFETY: EventPool only contains a Mutex<Vec<HANDLE>>. HANDLEs are pointer-sized
// values that are safe to send between threads. The Mutex provides synchronization.
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
            unsafe {
                ResetEvent(h);
            }
            return Ok(h);
        }
        drop(events);

        let h = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if h.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(h)
        }
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

/// # Safety
/// The caller must ensure `handle` is a valid, open file/pipe handle opened
/// with FILE_FLAG_OVERLAPPED.
pub unsafe fn async_read(handle: HANDLE, buf: &mut [u8], pool: &EventPool) -> io::Result<usize> {
    let event_guard = EventGuard::new(pool)?;

    let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
    overlapped.hEvent = event_guard.handle;

    let mut bytes_read: u32 = 0;
    let result = ReadFile(
        handle,
        buf.as_mut_ptr().cast(),
        buf.len() as u32,
        &mut bytes_read,
        &mut overlapped,
    );

    if result != 0 {
        return Ok(bytes_read as usize);
    }

    let err = GetLastError();
    if err != ERROR_IO_PENDING {
        return Err(io::Error::from_raw_os_error(err as i32));
    }

    let wait_result = WaitForSingleObject(event_guard.handle, INFINITE);
    if wait_result != WAIT_OBJECT_0 {
        return Err(io::Error::last_os_error());
    }

    let mut transferred: u32 = 0;
    let success = GetOverlappedResult(handle, &overlapped, &mut transferred, 0);
    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(transferred as usize)
}

/// # Safety
/// The caller must ensure `handle` is a valid, open file/pipe handle opened
/// with FILE_FLAG_OVERLAPPED.
pub unsafe fn async_write(handle: HANDLE, buf: &[u8], pool: &EventPool) -> io::Result<usize> {
    let event_guard = EventGuard::new(pool)?;

    let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
    overlapped.hEvent = event_guard.handle;

    let mut bytes_written: u32 = 0;
    let result = WriteFile(
        handle,
        buf.as_ptr().cast(),
        buf.len() as u32,
        &mut bytes_written,
        &mut overlapped,
    );

    if result != 0 {
        return Ok(bytes_written as usize);
    }

    let err = GetLastError();
    if err != ERROR_IO_PENDING {
        return Err(io::Error::from_raw_os_error(err as i32));
    }

    let wait_result = WaitForSingleObject(event_guard.handle, INFINITE);
    if wait_result != WAIT_OBJECT_0 {
        return Err(io::Error::last_os_error());
    }

    let mut transferred: u32 = 0;
    let success = GetOverlappedResult(handle, &overlapped, &mut transferred, 0);
    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(transferred as usize)
}
