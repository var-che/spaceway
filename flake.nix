{
  description = "Spaceway - Privacy-preserving decentralized communication platform";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        # Pin Rust version
        rustVersion = "1.75.0";
        rust = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # System dependencies
        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          perl
        ];

        buildInputs = with pkgs; [
          openssl
          sqlite
          # RocksDB dependencies
          rocksdb
          zlib
          bzip2
          lz4
          zstd
          snappy
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          pkgs.libiconv
        ];

      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          packages = with pkgs; [
            rust
            cargo-watch
            cargo-edit
            cargo-udeps
            cargo-audit
            git
          ];

          shellHook = ''
            echo "╔════════════════════════════════════════╗"
            echo "║   Descord Development Environment     ║"
            echo "╚════════════════════════════════════════╝"
            echo ""
            echo "Rust version: ${rustVersion}"
            echo "System: ${system}"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo test           - Run tests"
            echo "  cargo run --bin descord -- Build and run CLI"
            echo "  cargo watch -x test  - Auto-run tests on changes"
            echo ""
            
            # Set environment variables for RocksDB
            export ROCKSDB_LIB_DIR="${pkgs.rocksdb}/lib"
            export SNAPPY_LIB_DIR="${pkgs.snappy}/lib"
            
            # Ensure cargo home is set
            export CARGO_HOME="''${CARGO_HOME:-$HOME/.cargo}"
            export PATH="$CARGO_HOME/bin:$PATH"
          '';

          # Environment variables for build
          RUST_BACKTRACE = "1";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };

        # Package the CLI application
        packages.spaceway-cli = pkgs.rustPlatform.buildRustPackage {
          pname = "spaceway";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Skip tests during build (they require network)
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Privacy-preserving decentralized communication platform";
            homepage = "https://github.com/yourusername/spaceway";
            license = licenses.mit;
            maintainers = [ ];
          };
        };

        # Default package
        packages.default = self.packages.${system}.spaceway-cli;

        # For running tests with proper environment
        apps.test = {
          type = "app";
          program = "${pkgs.writeShellScript "test-descord" ''
            export RUST_BACKTRACE=1
            ${rust}/bin/cargo test --features test-utils "$@"
          ''}";
        };
      }
    );
}
