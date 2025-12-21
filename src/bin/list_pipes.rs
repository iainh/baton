use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "list_pipes", version = env!("CARGO_PKG_VERSION"))]
#[command(about = "List Windows named pipes with optional glob pattern filtering")]
struct Args {
    /// Glob pattern to filter pipe names (e.g., "docker_*", "gpg-agent")
    #[arg(short, long)]
    filter: Option<String>,

    /// Enable verbose logging for debugging
    #[arg(short, long)]
    verbose: bool,

    /// Output full pipe paths (e.g., \\.\pipe\name) instead of just names
    #[arg(short, long)]
    path: bool,
}

#[cfg(windows)]
fn main() -> Result<()> {
    use baton::win::{enumerate_pipes, filter_pipes};

    let args = Args::parse();

    // Initialize logging, respecting existing RUST_LOG if set
    if args.verbose && std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "list_pipes=debug,baton::win=debug,warn");
    }
    let _ = env_logger::try_init();

    // Enumerate all pipes
    let pipes = enumerate_pipes().context("failed to enumerate Windows named pipes via WMI")?;

    // Filter by pattern if provided
    let filtered = filter_pipes(pipes, args.filter.as_deref()).with_context(|| {
        format!(
            "invalid glob pattern for --filter: {:?}",
            args.filter.as_deref()
        )
    })?;

    // Output one pipe per line
    for pipe in filtered {
        if args.path {
            println!(r"\\.\pipe\{}", pipe.name);
        } else {
            println!("{}", pipe.name);
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("list_pipes is Windows-only (target a Windows triple to run).");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_args_valid() {
        Args::command().debug_assert();
    }

    #[test]
    fn test_parse_no_args() {
        let args = Args::try_parse_from(["list_pipes"]).unwrap();
        assert!(args.filter.is_none());
        assert!(!args.verbose);
    }

    #[test]
    fn test_parse_filter() {
        let args = Args::try_parse_from(["list_pipes", "-f", "docker_*"]).unwrap();
        assert_eq!(args.filter.as_deref(), Some("docker_*"));
    }

    #[test]
    fn test_parse_filter_long() {
        let args = Args::try_parse_from(["list_pipes", "--filter", "gpg-*"]).unwrap();
        assert_eq!(args.filter.as_deref(), Some("gpg-*"));
    }

    #[test]
    fn test_parse_verbose() {
        let args = Args::try_parse_from(["list_pipes", "-v"]).unwrap();
        assert!(args.verbose);
    }

    #[test]
    fn test_parse_verbose_long() {
        let args = Args::try_parse_from(["list_pipes", "--verbose"]).unwrap();
        assert!(args.verbose);
    }

    #[test]
    fn test_parse_filter_and_verbose() {
        let args = Args::try_parse_from(["list_pipes", "-v", "-f", "agent*"]).unwrap();
        assert_eq!(args.filter.as_deref(), Some("agent*"));
        assert!(args.verbose);
    }

    #[test]
    fn test_parse_path() {
        let args = Args::try_parse_from(["list_pipes", "-p"]).unwrap();
        assert!(args.path);
    }

    #[test]
    fn test_parse_path_long() {
        let args = Args::try_parse_from(["list_pipes", "--path"]).unwrap();
        assert!(args.path);
    }

    #[test]
    fn test_parse_all_flags() {
        let args = Args::try_parse_from(["list_pipes", "-v", "-p", "-f", "agent*"]).unwrap();
        assert_eq!(args.filter.as_deref(), Some("agent*"));
        assert!(args.verbose);
        assert!(args.path);
    }

    #[test]
    fn test_version_flag() {
        let result = Args::try_parse_from(["list_pipes", "--version"]);
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_help_flag() {
        let result = Args::try_parse_from(["list_pipes", "--help"]);
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }
}
