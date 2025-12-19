# Command-Line Interface Specification

## Basic Invocation

```bash
npiperelay.exe [flags] <pipe-name>
```

## Positional Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<pipe-name>` | Yes | The name of the Windows named pipe or Assuan socket to connect to |

### Pipe Name Formats

- **Named pipe format**: `//./pipe/docker_engine` or `\\.\pipe\docker_engine`
- **Assuan socket format** (with `-a` flag): File path to Assuan socket file

## Flag Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-p` | Boolean | false | Poll every 200ms until the named pipe exists and is not busy. Useful when the pipe may not be immediately available. |
| `-l` | Boolean | false | When polling (`-p`), limit attempts to 300 (approximately 60 seconds) instead of retrying indefinitely. |
| `-s` | Boolean | false | Send a 0-byte message to the pipe after EOF on stdin. Signals to the pipe server that no more data is coming. Essential for message-mode pipes. |
| `-ep` | Boolean | false | Terminate immediately on EOF when reading from the pipe, even if there is pending data to write to stdin. |
| `-ei` | Boolean | false | Terminate immediately on EOF when reading from stdin, even if there is pending data from the pipe. |
| `-bg` | Boolean | false | Hide the console window and run the process in the background. Uses Windows API to hide the console. |
| `-a` | Boolean | false | Treat the target as an Assuan file socket (used by GnuPG/ssh-agent). Special handling for Assuan protocol format. |
| `-v` | Boolean | false | Enable verbose output on stderr for debugging. Logs connection status and data flow events. |

## Help and Version Output

Running `npiperelay.exe` without arguments or with invalid arguments displays:

```
npiperelay v<version>
  commit <git-sha>
  build date <RFC3339-date>
  built by <builder-info>
  built with <go-version>

usage:
```

## Flag Behavior Matrix

| Flag | Stdin EOF | Pipe EOF | Result |
|------|-----------|----------|--------|
| (none) | Exit(0)* | Exit(0)* | Wait for both |
| `-ei` | Exit(0) | Exit(0)* | Exit on stdin EOF |
| `-ep` | Exit(0)* | Exit(0) | Exit on pipe EOF |
| `-ei -ep` | Exit(0) | Exit(0) | Exit on either EOF |
| `-s` | Send 0-byte | — | Signal end of stream |
| `-p` | — | — | Retry connection on busy |
| `-l` | — | — | Limit retry attempts to 300 |
| `-a` | — | — | Use Assuan protocol mode |
| `-bg` | — | — | Hide console window |
| `-v` | — | — | Enable verbose logging |

*Exit triggered by `ERROR_BROKEN_PIPE` or `ERROR_PIPE_NOT_CONNECTED` from ReadFile

## Common Flag Combinations

| Use Case | Recommended Flags |
|----------|-------------------|
| Docker relay | `-ep -s` |
| SSH agent | `-ei -s` |
| GnuPG agent | `-ei -ep -a` |
| MySQL named pipe | `-p -l -s` |
| Hyper-V serial | `-p -s` |
| Debugging | `-v` |
