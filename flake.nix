{
  description = "Baton - Rust port of npiperelay for Windows named pipes";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain with Windows cross-compilation target
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
          targets = [ "x86_64-pc-windows-gnu" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.cargo-watch
            pkgs.cargo-edit
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # macOS cross-compilation to Windows
            pkgs.pkgsCross.mingwW64.stdenv.cc
            pkgs.pkgsCross.mingwW64.windows.pthreads
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.pkgsCross.mingwW64.stdenv.cc
            pkgs.pkgsCross.mingwW64.windows.pthreads
          ];

          shellHook = ''
            echo "Baton development environment"
            echo "  cargo build                              - Build for host (stub)"
            echo "  cargo build --target x86_64-pc-windows-gnu - Cross-compile for Windows"
            echo ""
          '';

          # Set up cross-compilation environment variables
          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
        };
      }
    );
}
