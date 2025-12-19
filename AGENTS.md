# AGENTS.md - Baton Project

## Project Overview

**Baton** is a Rust port of [npiperelay](https://github.com/albertony/npiperelay), a Windows-only tool that enables access to Windows named pipes from WSL (Windows Subsystem for Linux). It relays data between stdin/stdout and Windows named pipes, typically used with `socat` to bridge Unix domain sockets to Windows named pipes.

## Build & Test Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Type checking
cargo check

# Run tests
cargo test

# Cross-compile for Windows from macOS/Linux
cargo build --target x86_64-pc-windows-msvc
```

## Project Structure

```
baton/
  Cargo.toml
  src/
    main.rs          # Entry point with platform gating
    cli.rs           # CLI parsing using clap
    logging.rs       # Logging initialization
    relay.rs         # Bidirectional relay implementation
    assuan.rs        # Assuan protocol for GnuPG/ssh-agent
    errors.rs        # Error types
    win/
      mod.rs         # Windows-specific module
      pipe.rs        # Named pipe connection & I/O
      overlapped.rs  # Event pool + async_io helper
      console.rs     # -bg console hiding
  docs/              # Documentation
```

## Key Dependencies

- `clap` (v4, derive feature) - CLI parsing
- `log` + `env_logger` - Logging
- `anyhow` + `thiserror` - Error handling
- `windows-sys` (v0.59) - Windows API FFI

## CLI Flags

| Flag | Description |
|------|-------------|
| `-p` | Poll until pipe is available (200ms interval) |
| `-l` | Limit polling to 300 attempts (~60s) |
| `-s` | Send 0-byte message on stdin EOF |
| `--ep` | Exit immediately on pipe EOF |
| `--ei` | Exit immediately on stdin EOF |
| `--bg` | Hide console window |
| `-a` | Assuan socket mode (for GnuPG) |
| `-v` | Verbose logging |

## Implementation Notes

- **Platform gating**: Use `#[cfg(windows)]` for Windows-specific code
- **Named pipes**: Use overlapped I/O with `CreateFileW`, `ReadFile`, `WriteFile`
- **Assuan mode**: Read port+nonce from file, connect via TCP to 127.0.0.1
- **EOF semantics**: Two threads coordinate via `AtomicBool` for `-ei`/`-ep` behavior
- **Error mapping**: Map `ERROR_BROKEN_PIPE` to `ErrorKind::BrokenPipe`

## Code Style

- Follow Rust 2021 edition idioms
- Use `anyhow::Result` for error propagation
- Prefer `windows-sys` over `windows` crate for lighter FFI
- Gate all Windows modules with `#[cfg(windows)]`
- Provide non-Windows stub that prints error and exits

## Testing

- Unit tests for CLI parsing and Assuan file parsing
- Integration tests (Windows only) with named pipe server
- Manual testing with socat for Docker, SSH agent, GnuPG use cases

## Documentation

See `docs/` for detailed specifications:
- `overview.md` - Architecture and use cases
- `cli.md` - CLI specification
- `data-flow.md` - Core data flow
- `windows-api.md` - Windows API usage
- `error-handling.md` - Error handling
- `implementation-plan.md` - Full implementation guidance
