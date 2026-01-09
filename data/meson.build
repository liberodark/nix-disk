subdir('icons')

gnome = import('gnome')

configure_file(
  input: 'org.glfos.nixdiskmanager.in',
  output: 'org.glfos.nixdiskmanager.policy',
  configuration: conf,
  install_dir: datadir / 'polkit-1' / 'actions'
)

configure_file(
  input: 'nix-disk.desktop.in',
  output: 'nix-disk.desktop',
  configuration: conf,
  install_dir: datadir / 'applications'
)