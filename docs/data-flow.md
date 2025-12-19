# Core Functionality and Data Flow

## Initialization Flow

1. **Argument Parsing**: Flags are parsed; exactly one positional argument (pipe name) is required
2. **Background Mode**: If `-bg` flag is set, console window is hidden via Windows API calls
3. **Connection Establishment**: Based on flags, connects to either:
   - **Standard Named Pipe** (`dialPipe`): Direct connection to Windows named pipe
   - **Assuan Socket** (`dialAssuan`): Reads port and nonce, then establishes TCP connection

## Connection Modes

### Standard Named Pipe Mode (default)

- Uses Windows `CreateFile` API with `FILE_FLAG_OVERLAPPED` for asynchronous I/O
- Flags used: `GENERIC_READ | GENERIC_WRITE`
- Security flags: `SECURITY_SQOS_PRESENT | SECURITY_ANONYMOUS`
- Optional polling on `ERROR_FILE_NOT_FOUND` or `ERROR_PIPE_BUSY`

### Assuan Mode (`-a` flag)

1. Reads initial configuration from the file specified:
   - First line: ASCII-encoded port number (e.g., "8000\n")
   - Next 16 bytes: Nonce value (binary)
2. Connects to TCP socket on localhost using the port number
3. Sends the nonce as the first 16 bytes of the TCP connection
4. This protocol is used by GnuPG's agent and ssh-agent

## Bidirectional Data Relay

```
┌─────────┐                                           ┌──────────────┐
│  stdin  │────────────────────────────────────────►  │              │
└─────────┘        io.Copy (stdin → pipe)             │              │
                                                      │  Named Pipe  │
┌─────────┐                                           │      or      │
│ stdout  │◄────────────────────────────────────────  │  TCP Socket  │
└─────────┘        io.Copy (pipe → stdout)            │              │
                                                      └──────────────┘
```

### Concurrent Operations

Two operations run concurrently:

1. **Main goroutine**: Copies data from named pipe to stdout using `io.Copy(os.Stdout, conn)`
2. **Background goroutine**: Copies data from stdin to named pipe using `io.Copy(conn, os.Stdin)`

## I/O Implementation Details

### Overlapped I/O Structure

The implementation uses asynchronous I/O for non-blocking operations:

- **Event Pool**: Maintains a thread-safe pool of Windows event objects for overlapped operations
- **asyncIo Method**: 
  1. Creates overlapped structure with event handle
  2. Executes I/O operation (Read/Write)
  3. If `ERROR_IO_PENDING`, waits on event using `GetOverlappedResult`
  4. Returns number of bytes transferred
- **Thread-safe Access**: Uses mutex to protect event pool concurrent access

### Stdin Copy Operation (background)

```
Read from stdin
    │
    ▼
Write to pipe ───► ERROR_BROKEN_PIPE? ───► Exit
    │
    │ (on stdin EOF)
    ▼
Send 0-byte if -s flag set
    │
    ▼
Check -ei flag ───► If set: Exit immediately
    │
    │ (if not set)
    ▼
Continue until pipe closes
```

### Pipe Read Operation (main)

```
Read from pipe
    │
    ▼
Write to stdout
    │
    │ (on pipe EOF / ERROR_BROKEN_PIPE)
    ▼
Check -ep flag ───► If set: Exit immediately
    │
    │ (if not set)
    ▼
Wait for stdin copy to finish
```

## Polling Mechanism

When `-p` flag is used:

| Parameter | Value |
|-----------|-------|
| Retry interval | 200ms |
| Default max attempts | Unlimited |
| With `-l` flag | 300 attempts (~60 seconds) |

### Retryable Errors

- `ERROR_FILE_NOT_FOUND`: Pipe doesn't exist yet
- `ERROR_PIPE_BUSY`: Pipe exists but all instances busy

### Non-retryable Errors

Any other Windows error causes immediate failure.

## Message-Mode Pipes

The `-s` flag implements proper shutdown for message-mode pipes:

- After stdin EOF, sends a **0-byte message** to the pipe
- In Windows message-mode pipes, zero-byte writes signal end-of-stream
- Ensures pipe server receives the shutdown signal
