# Baton

A Rust port of [npiperelay](https://github.com/albertony/npiperelay) — a Windows-only tool that enables access to Windows named pipes from WSL (Windows Subsystem for Linux).

Baton relays data between stdin/stdout and Windows named pipes, typically used with `socat` to bridge Unix domain sockets to Windows named pipes.

## Use Cases

- Connect Docker for Windows daemon to the Linux Docker client in WSL
- Connect to Windows SSH agent (`ssh-agent`) from WSL
- Connect to GnuPG agents running in Windows
- Access MySQL Server running as a Windows service via named pipes
- Connect to Hyper-V Linux VM serial consoles

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/baton.exe`.

## Usage

```bash
baton [FLAGS] <pipe-name>
```

### Flags

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

### Examples

**Docker relay:**
```bash
socat UNIX-LISTEN:/var/run/docker.sock,fork EXEC:"baton.exe -ep -s //./pipe/docker_engine"
```

**SSH agent relay:**
```bash
socat UNIX-LISTEN:$SSH_AUTH_SOCK,fork EXEC:"baton.exe -ei -s //./pipe/openssh-ssh-agent"
```

**GnuPG agent (Assuan protocol):**
```bash
socat UNIX-LISTEN:~/.gnupg/S.gpg-agent,fork EXEC:'baton.exe -ei -ep -a "C:/Users/.../S.gpg-agent"'
```

## Based On

This project is a Rust reimplementation of [npiperelay](https://github.com/albertony/npiperelay) by [albertony](https://github.com/albertony), originally written in Go.

## License

See [npiperelay](https://github.com/albertony/npiperelay) for the original project's license.
