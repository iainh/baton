use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "baton", version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Relay data between stdin/stdout and Windows named pipes")]
pub struct CliArgs {
    /// Poll every 200ms until the named pipe exists and is not busy
    #[arg(short = 'p')]
    pub poll: bool,

    /// When polling, limit attempts to 300 (~60 seconds)
    #[arg(short = 'l')]
    pub limited_poll: bool,

    /// Send a 0-byte message to the pipe after EOF on stdin
    #[arg(short = 's')]
    pub send_zero: bool,

    /// Exit immediately on EOF when reading from the pipe
    #[arg(long = "ep")]
    pub exit_on_pipe_eof: bool,

    /// Exit immediately on EOF when reading from stdin
    #[arg(long = "ei")]
    pub exit_on_stdin_eof: bool,

    /// Hide the console window and run in the background
    #[arg(long = "bg")]
    pub bg: bool,

    /// Treat the target as an Assuan file socket (for GnuPG)
    #[arg(short = 'a')]
    pub assuan: bool,

    /// Enable verbose output on stderr for debugging
    #[arg(short = 'v')]
    pub verbose: bool,

    /// Named pipe name or Assuan socket path
    pub pipe_name: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub pipe_name: String,
    pub poll: bool,
    pub limited_poll: bool,
    pub send_zero: bool,
    pub exit_on_pipe_eof: bool,
    pub exit_on_stdin_eof: bool,
    pub bg: bool,
    pub assuan: bool,
    pub verbose: bool,
}

impl From<CliArgs> for Config {
    fn from(args: CliArgs) -> Self {
        Config {
            pipe_name: args.pipe_name,
            poll: args.poll,
            limited_poll: args.limited_poll,
            send_zero: args.send_zero,
            exit_on_pipe_eof: args.exit_on_pipe_eof,
            exit_on_stdin_eof: args.exit_on_stdin_eof,
            bg: args.bg,
            assuan: args.assuan,
            verbose: args.verbose,
        }
    }
}

pub fn parse() -> Config {
    CliArgs::parse().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_args_valid() {
        CliArgs::command().debug_assert();
    }

    #[test]
    fn test_parse_basic() {
        let args = CliArgs::try_parse_from(["baton", "//./pipe/test"]).unwrap();
        assert_eq!(args.pipe_name, "//./pipe/test");
        assert!(!args.poll);
        assert!(!args.verbose);
    }

    #[test]
    fn test_parse_all_flags() {
        let args = CliArgs::try_parse_from([
            "baton", "-p", "-l", "-s", "--ep", "--ei", "--bg", "-a", "-v", "//./pipe/test",
        ])
        .unwrap();
        assert!(args.poll);
        assert!(args.limited_poll);
        assert!(args.send_zero);
        assert!(args.exit_on_pipe_eof);
        assert!(args.exit_on_stdin_eof);
        assert!(args.bg);
        assert!(args.assuan);
        assert!(args.verbose);
    }

    #[test]
    fn test_parse_missing_pipe_name() {
        let result = CliArgs::try_parse_from(["baton"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_args() {
        let args = CliArgs::try_parse_from(["baton", "-p", "-v", "//./pipe/test"]).unwrap();
        let config: Config = args.into();
        assert_eq!(config.pipe_name, "//./pipe/test");
        assert!(config.poll);
        assert!(config.verbose);
        assert!(!config.limited_poll);
        assert!(!config.assuan);
    }

    #[test]
    fn test_parse_windows_style_path() {
        let args = CliArgs::try_parse_from(["baton", "\\\\.\\pipe\\openssh-ssh-agent"]).unwrap();
        assert_eq!(args.pipe_name, "\\\\.\\pipe\\openssh-ssh-agent");
    }

    #[test]
    fn test_parse_unix_style_path() {
        let args = CliArgs::try_parse_from(["baton", "//./pipe/docker_engine"]).unwrap();
        assert_eq!(args.pipe_name, "//./pipe/docker_engine");
    }

    #[test]
    fn test_parse_assuan_socket_path() {
        let args =
            CliArgs::try_parse_from(["baton", "-a", "C:\\Users\\test\\AppData\\Roaming\\gnupg\\S.gpg-agent"])
                .unwrap();
        assert!(args.assuan);
        assert!(args.pipe_name.contains("gnupg"));
    }

    #[test]
    fn test_version_flag() {
        let result = CliArgs::try_parse_from(["baton", "--version"]);
        assert!(result.is_err()); // --version causes early exit
    }

    #[test]
    fn test_help_flag() {
        let result = CliArgs::try_parse_from(["baton", "--help"]);
        assert!(result.is_err()); // --help causes early exit
    }

    #[test]
    fn test_unknown_flag() {
        let result = CliArgs::try_parse_from(["baton", "--unknown", "//./pipe/test"]);
        assert!(result.is_err());
    }
}
