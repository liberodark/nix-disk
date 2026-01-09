{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        with pkgs;
        {
          packages = rec {
            nix-disk = pkgs.callPackage ./package.nix {};
            default = nix-disk;
          };
          devShells.default = mkShell {
            buildInputs = with pkgs; [
              rustc
              cargo
              rust-analyzer
              clippy
              rustfmt
              blueprint-compiler
              meson
              ninja
              libadwaita
              adwaita-icon-theme
              gtk4
              librsvg
              pkg-config
              glib
              gobject-introspection
              polkit
              parted
              e2fsprogs
              util-linux
            ];

            # Environment variables for development
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          };
        }
      );
}
