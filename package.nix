{
  lib,
  stdenv,
  rustPlatform,
  fetchFromGitHub,
  wrapGAppsHook4,
  meson,
  ninja,
  pkg-config,
  glib,
  desktop-file-utils,
  gettext,
  librsvg,
  blueprint-compiler,
  appstream-glib,
  libadwaita,
  gtk4,
  polkit,
  gobject-introspection,
  parted,
  e2fsprogs,
  util-linux,
  cargo,
  rustc,
}:

stdenv.mkDerivation rec {
  pname = "nix-disk";
  version = (lib.importTOML ./Cargo.toml).package.version;

  src = lib.cleanSource ./.;

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    appstream-glib
    blueprint-compiler
    desktop-file-utils
    gettext
    glib
    gobject-introspection
    meson
    ninja
    wrapGAppsHook4
    pkg-config
    rustPlatform.cargoSetupHook
    cargo
    rustc
  ];

  buildInputs = [
    gtk4
    libadwaita
    glib
    librsvg
    polkit
    e2fsprogs
    util-linux
  ];

  # Set environment variables for build
  LOCALE_DIR = "${placeholder "out"}/share/locale";

  # Pass absolute paths to the Rust binary via environment variables at build time
  PARTED_PATH = "${parted}/bin/parted";
  MKFS_EXT4_PATH = "${e2fsprogs}/bin/mkfs.ext4";

  # Wrap the binary to include runtime dependencies in PATH
  preFixup = ''
    gappsWrapperArgs+=(
      --prefix PATH : "${lib.makeBinPath [ parted e2fsprogs util-linux ]}"
      --set PARTED_BIN "${parted}/bin/parted"
      --set MKFS_EXT4_BIN "${e2fsprogs}/bin/mkfs.ext4"
    )
  '';

  meta = with lib; {
    description = "A simple GUI to manage disks on NixOS (Rust version)";
    license = licenses.gpl3Plus;
    mainProgram = "nix-disk";
  };
}
