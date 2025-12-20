use glob::{Pattern, PatternError};
use log::debug;

/// Represents an enumerated Windows named pipe
#[derive(Debug, Clone)]
pub struct EnumeratedPipe {
    pub name: String,
}

/// Query all Windows named pipes via WMI
#[cfg(windows)]
pub fn enumerate_pipes() -> anyhow::Result<Vec<EnumeratedPipe>> {
    use serde::Deserialize;
    use wmi::{COMLibrary, WMIConnection};

    /// WMI Win32_NamedPipeFile struct for deserialization
    #[derive(Debug, Clone, Deserialize)]
    struct NamedPipeFileWmi {
        name: String,
    }

    let com_lib = COMLibrary::new()?;
    let wmi_con = WMIConnection::new(com_lib)?;

    let results: Vec<NamedPipeFileWmi> = wmi_con.query()?;

    debug!("WMI returned {} named pipes", results.len());

    Ok(results
        .into_iter()
        .map(|pipe_file| EnumeratedPipe {
            name: extract_pipe_name(&pipe_file.name),
        })
        .collect())
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

/// Extract pipe name from WMI path
fn extract_pipe_name(path: &str) -> String {
    // Paths may look like:
    // - "\\.\pipe\pipename"
    // - "Global\pipename"
    // - "\Device\NamedPipe\pipename"
    path.rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pipe_name_windows_style() {
        let name = extract_pipe_name(r"\.\pipe\docker_engine");
        assert_eq!(name, "docker_engine");
    }

    #[test]
    fn test_extract_pipe_name_global() {
        let name = extract_pipe_name(r"Global\gpg-agent");
        assert_eq!(name, "gpg-agent");
    }

    #[test]
    fn test_extract_pipe_name_device() {
        let name = extract_pipe_name(r"\Device\NamedPipe\openssh-ssh-agent");
        assert_eq!(name, "openssh-ssh-agent");
    }

    #[test]
    fn test_extract_pipe_name_simple() {
        let name = extract_pipe_name("simple_pipe");
        assert_eq!(name, "simple_pipe");
    }

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
