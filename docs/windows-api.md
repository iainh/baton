# Windows Named Pipe Handling and API Usage

## Named Pipe Connection

### CreateFile Parameters

```
CreateFile(
    pipe_path,                              // Path to named pipe
    GENERIC_READ | GENERIC_WRITE,           // Access mode
    0,                                       // No sharing
    NULL,                                    // Default security
    OPEN_EXISTING,                           // Must exist
    FILE_FLAG_OVERLAPPED |                   // Async I/O
    SECURITY_SQOS_PRESENT |                  // Security context
    SECURITY_ANONYMOUS,                      // Anonymous auth
    NULL                                     // No template
)
```

### Key Characteristics

| Characteristic | Value |
|----------------|-------|
| Access mode | Read and write |
| Sharing | None (exclusive access) |
| I/O mode | Overlapped (asynchronous) |
| Security | Anonymous (no user identity verification) |

## Critical Windows API Functions

### Named Pipe Operations

| Function | Purpose |
|----------|---------|
| `CreateFile` | Open named pipe for reading/writing |
| `ReadFile` | Read data from pipe (with OVERLAPPED) |
| `WriteFile` | Write data to pipe (with OVERLAPPED) |
| `CloseHandle` | Close pipe handle |

### Overlapped I/O Functions

| Function | Purpose |
|----------|---------|
| `CreateEvent` | Create event object for async completion |
| `GetOverlappedResult` | Wait for and retrieve async operation result |
| `WaitForSingleObject` | Wait on event handle (alternative approach) |

### Console Functions (for -bg flag)

| Function | Purpose |
|----------|---------|
| `GetConsoleWindow` | Get handle to console window |
| `ShowWindow` | Hide or show the console window |

### Winsock Functions (for Assuan mode)

| Function | Purpose |
|----------|---------|
| `socket` / `WSASocket` | Create TCP socket |
| `connect` | Connect to localhost on specified port |
| `send` / `recv` | Transfer data over TCP |
| `closesocket` | Close socket |

## Windows Error Codes

### Connection Errors

| Error Code | Name | Meaning |
|------------|------|---------|
| 2 | `ERROR_FILE_NOT_FOUND` | Pipe does not exist (retryable with -p) |
| 231 | `ERROR_PIPE_BUSY` | All pipe instances busy (retryable with -p) |
| 5 | `ERROR_ACCESS_DENIED` | Permission denied |

### I/O Errors

| Error Code | Name | Meaning |
|------------|------|---------|
| 997 | `ERROR_IO_PENDING` | Async operation in progress (not an error) |
| 109 | `ERROR_BROKEN_PIPE` | Pipe closed by remote end |
| 233 | `ERROR_PIPE_NOT_CONNECTED` | Pipe disconnected |

## Overlapped I/O Implementation

### Event Pool Management

The implementation maintains a pool of reusable event objects:

```
struct EventPool {
    events: Vec<HANDLE>,
    mutex: Mutex,
}

fn get_event() -> HANDLE {
    lock(mutex)
    if events.is_empty() {
        return CreateEvent(NULL, TRUE, FALSE, NULL)  // Manual reset, not signaled
    }
    return events.pop()
}

fn put_event(h: HANDLE) {
    lock(mutex)
    events.push(h)
}
```

### Async I/O Pattern

```
fn async_io(operation, buffer) -> Result<usize> {
    let event = get_event();
    
    let overlapped = OVERLAPPED {
        hEvent: event,
        ..default()
    };
    
    let result = operation(&overlapped);
    
    if result == ERROR_IO_PENDING {
        // Wait for completion
        GetOverlappedResult(handle, &overlapped, &bytes_transferred, TRUE);
    }
    
    put_event(event);
    return bytes_transferred;
}
```

## Security Considerations

### SECURITY_SQOS_PRESENT | SECURITY_ANONYMOUS

These flags specify:
- `SECURITY_SQOS_PRESENT`: Security Quality of Service flags are present
- `SECURITY_ANONYMOUS`: Use anonymous security context

This means:
- The pipe client does not impersonate the caller
- No user credentials are passed to the pipe server
- Minimizes security exposure

## Pipe Types

Windows named pipes can operate in two modes:

### Byte Mode (default)
- Data is treated as a stream of bytes
- No message boundaries

### Message Mode
- Data is treated as discrete messages
- Zero-byte writes signal message boundaries
- The `-s` flag is essential for proper operation

## Platform-Specific Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `SECURITY_SQOS_PRESENT` | 0x00100000 | QoS flags present |
| `SECURITY_ANONYMOUS` | 0x00000000 | Anonymous impersonation |
| `FILE_FLAG_OVERLAPPED` | 0x40000000 | Async I/O |
| `GENERIC_READ` | 0x80000000 | Read access |
| `GENERIC_WRITE` | 0x40000000 | Write access |
| `OPEN_EXISTING` | 3 | File must exist |
