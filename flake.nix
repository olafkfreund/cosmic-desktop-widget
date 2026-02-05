{
  description = "Desktop widgets using Wayland Layer Shell for COSMIC Desktop";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        # Native build inputs
        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          just
        ];

        # Runtime dependencies
        buildInputs = with pkgs; [
          wayland
          wayland-protocols
          libxkbcommon
          fontconfig
          freetype
          openssl
          # Audio support (for alarms and notifications)
          alsa-lib
        ];

      in {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;
          
          shellHook = ''
            echo "╔═══════════════════════════════════════════════╗"
            echo "║  COSMIC Desktop Widget Dev Shell 🎨          ║"
            echo "║  Layer Shell • Rust • Wayland                 ║"
            echo "╚═══════════════════════════════════════════════╝"
            echo ""
            echo "Available commands:"
            echo "  just build       - Build the project"
            echo "  just run         - Run the widget"
            echo "  just test        - Run tests"
            echo "  just check       - Run clippy"
            echo ""
            
            export RUST_LOG=info
            export WAYLAND_DISPLAY=wayland-0
          '';
          
          # Environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };

        # Package definition
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cosmic-desktop-widget";
          version = "0.1.0";
          
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          inherit nativeBuildInputs buildInputs;
          
          # Tests require Wayland
          doCheck = false;
          
          meta = with pkgs.lib; {
            description = "Desktop widgets using Wayland Layer Shell for COSMIC Desktop";
            homepage = "https://github.com/yourusername/cosmic-desktop-widget";
            license = licenses.gpl3Only;
            maintainers = [ ];
            platforms = platforms.linux;
          };
        };

        # Run the widget directly
        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/cosmic-desktop-widget";
        };
      }
    );
}
