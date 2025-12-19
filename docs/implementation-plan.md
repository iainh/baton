# Rust Implementation Plan for baton

This document provides a comprehensive implementation plan for a Rust port of npiperelay (named "baton"), suitable for another LLM to implement.

## TL;DR

Implement a single `baton` binary crate that mirrors the Go npiperelay tool's behavior, using `clap` for CLI, `windows-sys` for Win32 APIs (CreateFile, ReadFile/WriteFile, events, console), and std `TcpStream` for Assuan. Build a small internal abstraction for "connection" (named pipe vs Assuan), plus a synchronous overlapped-I/O wrapper with an event pool. Drive the bidirectional relay with two threads and a simple coordination state to honor `-ei`/`-ep` semantics.

**Effort**: L (1–2 days) for a careful, correct port; XL if you also implement advanced cancellation and exhaustive tests.

---

## 1. Project Structure & Cargo.toml

### Repository Layout

```text
baton/
  Cargo.toml
  src/
    main.rs
    cli.rs
    logging.rs
    relay.rs
    assuan.rs
    win/
      mod.rs
      pipe.rs       // named pipe open + Read/Write via overlapped
      overlapped.rs // event pool + async_io helper
      console.rs    // -bg implementation
    errors.rs
```

### Cargo.toml

```toml
[package]
name = "baton"
version = "0.1.0"
edition = "2021"

[dependencies]
# CLI parsing
clap = { version = "4", features = ["derive"] }

# Logging (simple; or you can just use eprintln! and a global verbose flag)
log = "0.4"
env_logger = "0.11"

# Error handling (optional but convenient)
anyhow = "1"
thiserror = "1"

# Windows APIs
windows-sys = { version = "0.59", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_Security",
    "Win32_System_IO",
    "Win32_System_Threading",
    "Win32_UI_WindowsAndMessaging",
] }

# For Assuan TCP connection and file I/O we just use std.
```

Platform gating in `Cargo.toml` is not strictly needed; prefer `cfg` in code so you can still cross-compile with `--target x86_64-pc-windows-msvc` from Linux/macOS.

---

## 2. Module Organization

### `src/main.rs`

Entry point with platform gating:

```rust
#[cfg(windows)]
fn main() {
    if let Err(e) = baton::real_main() {
        eprintln!("baton error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("baton is Windows-only (target a Windows triple to run).");
    std::process::exit(1);
}
```

### `cli.rs`

Defines a `Cli` struct using `clap::Parser`:

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub pipe_name: String,
    pub poll: bool,
    pub limited_poll: bool,
    pub send_zero: bool,
    pub exit_on_pipe_eof: bool,
    pub exit_on_stdin_eof: bool,
    pub bg: bool,
    pub assuan: bool,
    pub verbose: bool,
}
```

### `logging.rs`

Simple helper to initialize logging:

```rust
pub fn init_logging(verbose: bool) {
    if verbose {
        std::env::set_var("RUST_LOG", "baton=debug");
    }
    let _ = env_logger::try_init();
}
```

### `win/mod.rs`

Windows-specific functionality (all gated with `#![cfg(windows)]`):

- `pipe.rs`: named pipe connection & I/O wrapper
- `overlapped.rs`: event pool + `async_io` helper
- `console.rs`: `-bg` support (hide console window)

### `assuan.rs`

Assuan protocol implementation for GnuPG/ssh-agent.

### `relay.rs`

Bidirectional relay implementation with EOF semantics.

---

## 3. Implementation Order

1. **CLI and stubs (S)**
   - Implement `cli.rs` using `clap`
   - `real_main()` with config parsing and stub connectors

2. **Logging & -v support (XS)**
   - Add `logging::init_logging(config.verbose)`

3. **Assuan connector (S–M)**
   - Read port and nonce from file
   - TCP connection with polling logic
   - Send nonce on connect

4. **Named pipe connect (M)**
   - `CreateFileW` with polling logic
   - Initially synchronous, then migrate to overlapped

5. **Relay loop (M)**
   - Blocking relay between stdin/stdout and connection
   - Respect `-s`, `-ei`, `-ep` semantics

6. **Overlapped I/O primitives (L)**
   - `EventPool` implementation
   - `async_read` / `async_write` wrappers

7. **NamedPipe Read/Write using overlapped (M)**
   - Add `FILE_FLAG_OVERLAPPED` support
   - Implement `Read`/`Write` traits

8. **Refine relay for strict EOF semantics (L)**
   - Coordination between two directions
   - Accurate `-ei`/`-ep` behavior

9. **-bg and polishing (XS)**
   - Console hiding via Windows API

10. **Testing & verification (M–L)**
    - Unit tests and integration tests

---

## 4. Key Rust Crates for Windows APIs

### `windows-sys`

Lightweight FFI to Win32, use modules:

- `Win32::Storage::FileSystem` - `CreateFileW`, `ReadFile`, `WriteFile`, flags
- `Win32::Foundation` - `HANDLE`, `CloseHandle`, `BOOL`, `GetLastError`
- `Win32::System::IO` - `OVERLAPPED`, `GetOverlappedResult`, `CancelIoEx`
- `Win32::System::Threading` - `CreateEventW`, `WaitForSingleObject`
- `Win32::UI::WindowsAndMessaging` - console hiding
- `Win32::Security` - `SECURITY_SQOS_PRESENT`, `SECURITY_ANONYMOUS`

For concurrency, use std `std::sync::{Arc, Mutex, atomic}`.

---

## 5. Component Implementation Guidance

### 5.1 CLI Parsing & Version Output

```rust
#[derive(clap::Parser, Debug)]
#[command(name = "baton", version = env!("CARGO_PKG_VERSION"))]
pub struct CliArgs {
    #[arg(short = 'p')]
    poll: bool,
    #[arg(short = 'l')]
    limited_poll: bool,
    #[arg(short = 's')]
    send_zero: bool,
    #[arg(long = "ep")]
    exit_on_pipe_eof: bool,
    #[arg(long = "ei")]
    exit_on_stdin_eof: bool,
    #[arg(long = "bg")]
    bg: bool,
    #[arg(short = 'a')]
    assuan: bool,
    #[arg(short = 'v')]
    verbose: bool,
    /// Named pipe name or Assuan socket path
    pub pipe_name: String,
}
```

Note: `clap` doesn't support `-ep`/`-ei` as bundled short flags directly; use long flags.

### 5.2 Background Mode (`-bg`)

```rust
pub fn hide_console_window() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetConsoleWindow, ShowWindow, SW_HIDE};

    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd != 0 {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}
```

### 5.3 Named Pipe Connection

`connect_named_pipe`:

1. Convert pipe path (UTF-8) to UTF-16 with trailing NUL
2. Accept both `//./pipe/foo` and `\\.\pipe\foo`
3. Call `CreateFileW` in a loop if `-p` set:
   - Desired access: `GENERIC_READ | GENERIC_WRITE`
   - Share mode: `0`
   - Creation disposition: `OPEN_EXISTING`
   - Flags: `FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_ANONYMOUS`
4. On error:
   - If `ERROR_FILE_NOT_FOUND` or `ERROR_PIPE_BUSY`:
     - With `-p`: Sleep 200ms, retry (max 300 with `-l`)
     - Without `-p`: Fail immediately
   - Else: Fatal error

### 5.4 Assuan Mode Connection

`connect_assuan(path, config)`:

1. Open file with `std::fs::File::open`
2. Read first line for port number (ASCII)
3. Parse `u16` port, validate range
4. Read exactly 16 bytes for nonce
5. `TcpStream::connect(("127.0.0.1", port))` with retry logic
6. Send nonce immediately via `write_all(&nonce)`

### 5.5 Overlapped I/O and Event Pool

#### EventPool

```rust
pub struct EventPool {
    inner: Mutex<Vec<HANDLE>>,
}

impl EventPool {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Vec::new()) }
    }

    pub fn get(&self) -> io::Result<HANDLE> {
        let mut events = self.inner.lock().unwrap();
        if let Some(h) = events.pop() {
            return Ok(h);
        }
        let h = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, std::ptr::null()) };
        if h == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(h)
        }
    }

    pub fn put(&self, h: HANDLE) {
        self.inner.lock().unwrap().push(h);
    }
}
```

#### Async Read Pattern

1. Acquire event from pool
2. Create `OVERLAPPED` with `hEvent = event`
3. Call `ReadFile`
4. If returns non-zero: use `bytes_read`
5. If `ERROR_IO_PENDING`: call `GetOverlappedResult` and wait
6. Always return event to pool (use Drop guard)

### 5.6 NamedPipe Type

```rust
pub struct NamedPipe {
    handle: HANDLE,
    pool: Arc<EventPool>,
}

impl Read for NamedPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        overlapped::async_read(self.handle, buf, &self.pool)
    }
}

impl Write for NamedPipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        overlapped::async_write(self.handle, buf, &self.pool)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle); }
    }
}
```

### 5.7 Bidirectional Relay & EOF Semantics

Create shared state:

```rust
struct State {
    stdin_done: AtomicBool,
    pipe_done: AtomicBool,
}
```

Two concurrent threads:

1. **stdin_to_pipe thread**:
   - Read from stdin, write to connection
   - On stdin EOF:
     - If `-s`: send 0-byte write
     - Mark `stdin_done`
     - If `-ei`: exit immediately

2. **pipe_to_stdout (main thread)**:
   - Read from connection, write to stdout
   - On pipe EOF/BrokenPipe:
     - Mark `pipe_done`
     - If `-ep`: exit immediately
     - Else: wait for stdin thread if not done

### 5.8 Error Handling

- Use `anyhow::Error` for error propagation
- For Windows errors: `io::Error::from_raw_os_error(GetLastError() as i32)`
- Map `ERROR_BROKEN_PIPE`, `ERROR_PIPE_NOT_CONNECTED` to `ErrorKind::BrokenPipe`
- Exit code 0 for graceful EOF, non-zero for fatal errors

---

## 6. Testing Strategy

### 6.1 Unit Tests

- **CLI parsing**: Test with various argument combinations
- **Assuan parser**: Test valid/invalid port and nonce files
- **Polling logic**: Abstract into testable function with mock errors

### 6.2 Windows Integration Tests

Use `#[cfg(windows)]` tests that:

1. Create a named pipe server in a separate thread
2. Launch relay and verify data transfer
3. Test EOF and `-s` behavior
4. Test polling with delayed server start
5. Test Assuan mode with TCP listener

### 6.3 Manual Testing

```bash
# Docker
socat UNIX-LISTEN:/var/run/docker.sock,fork EXEC:"baton.exe -ep -s //./pipe/docker_engine"

# SSH agent
socat UNIX-LISTEN:$SSH_AUTH_SOCK,fork EXEC:"baton.exe -ei -s //./pipe/openssh-ssh-agent"

# GnuPG
socat UNIX-LISTEN:~/.gnupg/S.gpg-agent,fork EXEC:'baton.exe -ei -ep -a "C:/Users/.../S.gpg-agent"'
```

---

## 7. Potential Pitfalls and Solutions

### 7.1 Overlapped I/O Correctness

**Pitfalls**:
- Stack-allocated `OVERLAPPED` outliving function
- Forgetting to read `GetLastError` immediately
- Leaking event handles

**Solutions**:
- Wait for completion before returning (stack is safe)
- Capture `GetLastError()` immediately after API call
- Use Drop guard for event handle return

### 7.2 HANDLE and Lifetime Management

**Pitfalls**:
- Double-closing handles
- Unsafe sharing across threads

**Solutions**:
- Single owner pattern with `Drop`
- Use `Arc` for shared handles with one reader + one writer

### 7.3 EOF and Error Code Handling

**Pitfalls**:
- Misinterpreting 0-byte read vs `ERROR_BROKEN_PIPE`
- Treating `ERROR_IO_PENDING` as fatal

**Solutions**:
- Treat 0-byte read as EOF (same as `ERROR_BROKEN_PIPE`)
- Only treat `ERROR_IO_PENDING` as "wait for completion"

### 7.4 Polling Behavior

**Pitfalls**:
- Wrong interval or busy-waiting
- Not honoring `-l` limit

**Solutions**:
- Use `std::thread::sleep(Duration::from_millis(200))`
- Track attempt counter, fail at 300 if `-l` set

### 7.5 Assuan File Parsing

**Pitfalls**:
- CRLF vs LF handling
- Reading more than needed, misaligning nonce

**Solutions**:
- Use `BufRead::read_line` and trim `\r\n`
- Use `read_exact` for exactly 16 bytes

### 7.6 Cross-Compilation

**Pitfalls**:
- Using Windows APIs in non-`cfg(windows)` code
- Breaking build for non-Windows targets

**Solutions**:
- Gate `mod win;` with `#[cfg(windows)]`
- Provide non-Windows stub `main`
- Cross-compile with `--target x86_64-pc-windows-msvc`

---

## 8. Advanced Path (Optional)

For more sophisticated implementation:

1. Replace manual overlapped I/O with `tokio::net::windows::named_pipe`
2. Use async tasks with `select!` for EOF semantics
3. Implement `CancelIoEx` for immediate I/O cancellation
4. Add property-based tests with `proptest`

For the initial port, the simpler synchronous-overlapped + threads design is recommended.
