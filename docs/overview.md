# npiperelay Overview

## Purpose

**npiperelay** is a Windows-only tool that enables access to Windows named pipes from the Windows Subsystem for Linux (WSL) by exposing stdin and stdout to relay data between processes running in WSL and Windows named pipes.

## Key Use Cases

- Connect Docker for Windows daemon to the Linux Docker client in WSL
- Access MySQL Server running as a Windows service via named pipes
- Connect interactively to Hyper-V Linux VM serial consoles
- Debug kernel of Hyper-V Linux VMs via gdb
- Connect to Windows SSH agent (`ssh-agent`) from WSL
- Access Windows network interfaces directly from WSL
- Connect to GnuPG agents running in Windows

## Architecture

The tool is designed to work in conjunction with `socat` (a multipurpose relay tool in Linux) to translate between Unix domain sockets in WSL and Windows named pipes. It's typically invoked as a subprocess by socat with data piped through stdin/stdout.

### Typical Usage Pattern

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              WSL                                        │
│                                                                         │
│  ┌──────────┐      ┌───────────────┐      ┌─────────────────────────┐  │
│  │  Client  │◄────►│ Unix Socket   │◄────►│         socat           │  │
│  │  (e.g.   │      │ (e.g.         │      │ (relays to subprocess)  │  │
│  │  docker) │      │ /var/run/     │      └───────────┬─────────────┘  │
│  └──────────┘      │ docker.sock)  │                  │                │
│                    └───────────────┘                  │ stdin/stdout   │
│                                                       ▼                │
├───────────────────────────────────────────────────────┬─────────────────┤
│                          Windows                      │                 │
│                                                       ▼                 │
│                                          ┌─────────────────────────┐   │
│  ┌──────────────────────────────┐        │     npiperelay.exe      │   │
│  │     Windows Named Pipe       │◄──────►│  (stdin/stdout relay)   │   │
│  │  (e.g. //./pipe/docker_engine│        └─────────────────────────┘   │
│  └──────────────────────────────┘                                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Example Commands

### Docker relay
```bash
socat UNIX-LISTEN:/var/run/docker.sock,fork EXEC:"npiperelay -ep -s //./pipe/docker_engine"
```

### SSH agent relay
```bash
socat UNIX-LISTEN:$SSH_AUTH_SOCK,fork EXEC:"npiperelay -ei -s //./pipe/openssh-ssh-agent"
```

### GnuPG agent with Assuan protocol
```bash
socat UNIX-LISTEN:~/.gnupg/S.gpg-agent,fork EXEC:'npiperelay -ei -ep -a "C:/Users/.../S.gpg-agent"'
```
