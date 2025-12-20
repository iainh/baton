pub fn init_logging(verbose: bool) {
    if verbose {
        std::env::set_var("RUST_LOG", "baton=debug");
    }
    // Intentionally ignore: try_init fails if called twice (e.g., in tests),
    // which is harmless and expected.
    let _ = env_logger::try_init();
}
