pub fn init_logging(verbose: bool) {
    if verbose {
        std::env::set_var("RUST_LOG", "baton=debug");
    }
    let _ = env_logger::try_init();
}
