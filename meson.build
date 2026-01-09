project('nix-disk', 'rust')

gnome = import('gnome')
cargo = find_program('cargo')

bindir = get_option('prefix') / get_option('bindir')
datadir = get_option('prefix') / get_option('datadir')
pkgdatadir = datadir / meson.project_name()
localedir = get_option('prefix') / get_option('localedir')

# Configuration data for substitutions
conf = configuration_data()
conf.set('bindir', bindir)

# Build Rust binary
cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'nix-disk',
  console: true,
  install: true,
  install_dir: bindir,
  command: [
    find_program('sh'),
    meson.project_source_root() / 'build-rust.sh',
    meson.project_source_root(),
    meson.project_build_root(),
    '@OUTPUT@'
  ]
)

# Process subdirectories
subdir('po')
subdir('data')
