use glob::{Pattern, PatternError};
use log::debug;

/// Represents an enumerated Windows named pipe
#[derive(Debug, Clone)]
pub struct EnumeratedPipe {
    pub name: String,
}

/// Enumerate all Windows named pipes via filesystem (\\.\pipe\*)
#[cfg(windows)]
pub fn enumerate_pipes() -> anyhow::Result<Vec<EnumeratedPipe>> {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW,
    };

    let search_path: Vec<u16> = r"\\.\pipe\*".encode_utf16().chain(std::iter::once(0)).collect();

    let mut find_data = MaybeUninit::<WIN32_FIND_DATAW>::uninit();
    let handle = unsafe { FindFirstFileW(search_path.as_ptr(), find_data.as_mut_ptr()) };

    if handle == INVALID_HANDLE_VALUE {
        let err = unsafe { GetLastError() };
        anyhow::bail!("FindFirstFileW failed with error code {}", err);
    }

    let mut pipes = Vec::new();
    let mut find_data = unsafe { find_data.assume_init() };

    loop {
        let name = wchar_to_string(&find_data.cFileName);
        if !name.is_empty() && name != "." && name != ".." {
            pipes.push(EnumeratedPipe { name });
        }

        let success = unsafe { FindNextFileW(handle, &mut find_data) };
        if success == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NO_MORE_FILES {
                break;
            }
            unsafe { FindClose(handle) };
            anyhow::bail!("FindNextFileW failed with error code {}", err);
        }
    }

    unsafe { FindClose(handle) };

    debug!("Filesystem enumeration found {} named pipes", pipes.len());

    Ok(pipes)
}

/// Convert null-terminated wide string to String
#[cfg(windows)]
fn wchar_to_string(wchars: &[u16]) -> String {
    let len = wchars.iter().position(|&c| c == 0).unwrap_or(wchars.len());
    String::from_utf16_lossy(&wchars[..len])
}

/// Filter pipes by glob pattern
pub fn filter_pipes(
    pipes: Vec<EnumeratedPipe>,
    pattern: Option<&str>,
) -> Result<Vec<EnumeratedPipe>, PatternError> {
    match pattern {
        None => Ok(pipes),
        Some(pattern_str) => {
            let pattern = Pattern::new(pattern_str)?;
            Ok(pipes
                .into_iter()
                .filter(|pipe| pattern.matches(&pipe.name))
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_pipes_no_pattern() {
        let pipes = vec![
            EnumeratedPipe {
                name: "docker_engine".to_string(),
            },
            EnumeratedPipe {
                name: "gpg-agent".to_string(),
            },
        ];
        let filtered = filter_pipes(pipes, None).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_pipes_glob_pattern() {
        let pipes = vec![
            EnumeratedPipe {
                name: "docker_engine".to_string(),
            },
            EnumeratedPipe {
                name: "docker_proxy".to_string(),
            },
            EnumeratedPipe {
                name: "gpg-agent".to_string(),
            },
        ];
        let filtered = filter_pipes(pipes, Some("docker_*")).unwrap();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|p| p.name == "docker_engine"));
        assert!(filtered.iter().any(|p| p.name == "docker_proxy"));
    }

    #[test]
    fn test_filter_pipes_question_mark() {
        let pipes = vec![
            EnumeratedPipe {
                name: "agent".to_string(),
            },
            EnumeratedPipe {
                name: "agents".to_string(),
            },
        ];
        let filtered = filter_pipes(pipes, Some("agent?")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "agents");
    }

    #[test]
    fn test_filter_pipes_no_matches() {
        let pipes = vec![
            EnumeratedPipe {
                name: "docker_engine".to_string(),
            },
            EnumeratedPipe {
                name: "gpg-agent".to_string(),
            },
        ];
        let filtered = filter_pipes(pipes, Some("mysql_*")).unwrap();
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_pipes_invalid_pattern() {
        let pipes = vec![EnumeratedPipe {
            name: "docker_engine".to_string(),
        }];
        let result = filter_pipes(pipes, Some("[invalid"));
        assert!(result.is_err());
    }
}
