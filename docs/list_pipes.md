# list_pipes - Named Pipes Enumeration Utility

`list_pipes` is a standalone Windows utility that enumerates all named pipes on the system and optionally filters them by glob pattern. It's useful for discovering available pipes for use with the main `baton` relay tool.

## Features

- **WMI-based enumeration**: Uses Windows Management Instrumentation to query all named pipes
- **Glob pattern filtering**: Filter pipes by shell-style glob patterns (e.g., `docker_*`, `gpg-agent`)
- **Production-ready**: Handles permission errors gracefully with warnings instead of failing
- **Simple output**: One pipe name per line for easy scripting and piping to other tools

## Usage

### List all named pipes
```bash
list_pipes.exe
```

### Filter pipes by pattern
```bash
# Show all Docker-related pipes
list_pipes.exe --filter "docker_*"

# Show all agent pipes
list_pipes.exe -f "*agent*"

# Show specific pipe
list_pipes.exe -f "openssh-ssh-agent"
```

### Verbose output for debugging
```bash
list_pipes.exe --verbose
list_pipes.exe -v --filter "docker_*"
```

## CLI Options

| Flag | Long Form | Description |
|------|-----------|-------------|
| `-f` | `--filter` | Glob pattern to filter pipe names |
| `-v` | `--verbose` | Enable verbose logging for debugging |

## Glob Pattern Examples

The filter uses standard glob patterns:

| Pattern | Matches |
|---------|---------|
| `*` | All pipes |
| `docker*` | `docker_engine`, `docker_proxy`, etc. |
| `*agent*` | `openssh-ssh-agent`, `gpg-agent`, `ssh-agent`, etc. |
| `gpg-agent` | Exact match: `gpg-agent` only |
| `[ab]*` | Pipes starting with 'a' or 'b' |
| `?ysql` | Single character followed by `ysql` (e.g., `mysql`) |

## Permission Handling

When `list_pipes` encounters a pipe that the current user cannot access due to permissions:

1. The pipe name is **skipped from output**
2. A **warning is logged** (visible with `-v`/`--verbose`)
3. Execution **continues normally** — no error exit code

This allows the tool to function in production environments where some pipes may be restricted.

## Output

The tool outputs one pipe name per line, without the leading `\.` or path prefix:

```
docker_engine
docker_proxy
gpg-agent
openssh-ssh-agent
postgres
```

This format is ideal for:
- Piping to other tools: `list_pipes.exe | findstr docker`
- Scripting: `for /F %p in ('list_pipes.exe -f "docker*"') do ...`
- Integration: Direct consumption by other applications

## Integration with Baton

Once you've discovered a pipe using `list_pipes`, you can relay to it using `baton`:

```bash
# Find and relay to Docker pipe
baton.exe //./pipe/docker_engine
```

## Implementation Details

- **Architecture**: Standalone binary (`src/bin/list_pipes.rs`)
- **WMI Class**: `Win32_NamedPipeFile` for enumeration
- **Dependencies**: 
  - `wmi` crate for WMI queries
  - `glob` crate for pattern matching
  - `clap` for CLI parsing
  - `log`/`env_logger` for logging

## Error Handling

### Access Denied
When a pipe exists but current user lacks read permissions:
- The pipe is not listed
- A warning is printed to stderr (with `-v` flag)
- Exit code remains 0

### WMI Connection Failure
If COM initialization or WMI connection fails:
- An error message is printed to stderr
- Exit code is 1

### Invalid Glob Pattern
If the filter pattern is invalid:
- All pipes are returned (filter is ignored)
- A warning is logged
- Exit code is 0
