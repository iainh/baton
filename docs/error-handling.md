# Error Handling Behavior

## Connection Errors

| Scenario | Behavior |
|----------|----------|
| Invalid pipe name (not found) | Immediate failure unless `-p` flag used |
| Pipe busy (all instances in use) | Immediate failure unless `-p` flag used |
| Polling timeout with `-l` flag | Failure after 300 attempts (~60 seconds) |
| Assuan protocol parse error | Fatal error (invalid port number or nonce length) |
| TCP connection failure (Assuan mode) | Retry with polling if `-p` flag set |

## Data Transfer Errors

| Condition | Behavior |
|-----------|----------|
| `ERROR_BROKEN_PIPE` | Pipe closed, exit immediately (data exhausted) |
| `ERROR_PIPE_NOT_CONNECTED` | Pipe disconnected, exit immediately |
| Other read/write errors | Log fatal error and exit with non-zero code |
| Stdin close failure | Log warning but continue |
| Stdout close failure | Log warning but continue |

## Graceful Shutdown Scenarios

### With `-ei` flag (exit on stdin EOF)

```
EOF on stdin
    │
    ▼
Send 0-byte message (if -s flag set)
    │
    ▼
Close stdin
    │
    ▼
Exit(0)

(Ignores any pending pipe data)
```

### Without `-ei` flag (default behavior)

```
EOF on stdin
    │
    ▼
Send 0-byte message (if -s flag set)
    │
    ▼
Wait for:
  • Pipe to close (ERROR_BROKEN_PIPE)
  • OR background goroutine to finish reading
    │
    ▼
Exit(0)
```

### With `-ep` flag (exit on pipe EOF)

```
EOF on pipe (ERROR_BROKEN_PIPE)
    │
    ▼
Exit(0) immediately

(Does NOT wait for stdin copy to finish)
```

## Error Logging

All errors are logged to stderr:

| Log Type | Function | When Used |
|----------|----------|-----------|
| Fatal error | `log.Fatalln()` | Connection failures, I/O errors |
| Warning | `log.Println()` | stdin/stdout close failures, 0-byte write failures |
| Verbose | `log.Println()` | With `-v` flag: connection and data flow events |

## Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success (graceful shutdown) |
| Non-zero | Fatal error occurred |

## Edge Cases

### Zero-Byte Read

A zero-byte read from the pipe indicates EOF:
- `ReadFile` returns success with 0 bytes transferred
- This triggers the same handling as `ERROR_BROKEN_PIPE`

### Concurrent Shutdown Race

When both stdin and pipe close simultaneously:
- The `-ei` and `-ep` flags determine which takes priority
- Without either flag, the program waits for both operations to complete
- With both flags, whichever completes first triggers exit

### Overlapped I/O Cancellation

When exiting before an async operation completes:
- Event handles must be properly cleaned up
- Pending I/O operations may need to be cancelled with `CancelIo`

### Assuan Mode Errors

| Error | Behavior |
|-------|----------|
| Cannot read port number | Fatal error |
| Port number not valid ASCII | Fatal error |
| Port number out of range | Fatal error |
| Nonce length != 16 bytes | Fatal error |
| TCP connection refused | Retry if polling, else fatal |
| Nonce send failure | Fatal error |

## Retry Logic Details

### Polling Implementation

```
attempt = 0
loop {
    result = try_connect()
    
    if result == SUCCESS {
        break
    }
    
    if result == ERROR_FILE_NOT_FOUND || result == ERROR_PIPE_BUSY {
        if -l flag && attempt >= 300 {
            fatal("polling limit reached")
        }
        sleep(200ms)
        attempt++
        continue
    }
    
    fatal(result)
}
```

### Polling Timeout Calculation

| Parameter | Value |
|-----------|-------|
| Sleep interval | 200ms |
| Max attempts with `-l` | 300 |
| Total timeout with `-l` | ~60 seconds |
| Without `-l` | Infinite |
