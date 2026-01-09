# Nix-disk

Une application GUI moderne en GTK4/Libadwaita pour gérer les configurations de montage de disques sur NixOS.

## Version 2.1 - Support du formatage de disques

Cette version ajoute des capacités de formatage de disques et corrige les problèmes de permissions.

### Nouveautés de la version 2.1

- **Formatage de disques** : Formatez des disques directement depuis l'application
  - Crée une table de partition GPT
  - Formate avec le système de fichiers ext4
  - Étiquettes de volume personnalisées
  - Configuration automatique des permissions pour l'utilisateur actuel
- **Correction des permissions** : Les disques nouvellement formatés sont maintenant automatiquement configurés avec la bonne propriété
  - Plus d'erreurs "Permission refusée" après le formatage
  - Les utilisateurs peuvent immédiatement écrire sur leurs disques formatés

### Version 2.0 - Réécriture en Rust

Réécriture complète en Rust pour de meilleures performances, sécurité mémoire et fiabilité :

- **Détection automatique de la langue** : Détecte et utilise automatiquement la locale de votre système
- **Détection des partitions manquantes** : Vérifie au démarrage les partitions configurées qui n'existent plus
  - Affiche un dialogue avec la liste des partitions manquantes
  - Option pour les supprimer automatiquement de la configuration

### Fonctionnalités principales

- Gestion visuelle des points de montage de disques sur NixOS
- Formatage de disques avec le système de fichiers ext4
- Lecture et modification de `/etc/nixos/hardware-configuration.nix`
- Options de montage automatiques spécifiques au système de fichiers (compression btrfs, permissions NTFS, etc.)
- Rebuild en direct avec `nixos-rebuild switch`
- Bannières d'état pour la progression et les erreurs de rebuild
- Support de plusieurs points de montage par partition
- Point de montage par défaut : `/media/` (emplacement convivial)

## Compilation

### Avec Nix (Recommandé)

```bash
# Compiler le paquet
nix build

# Exécuter directement
nix run

# Entrer dans l'environnement de développement
nix develop
```

### Avec Cargo

```bash
# Entrer d'abord dans l'environnement de développement
nix develop

# Compiler
cargo build --release

# Exécuter
cargo run --release
```

## Prérequis

- NixOS avec `/etc/nixos/hardware-configuration.nix`
- Privilèges root (via sudo/polkit) pour :
  - Opérations `nixos-rebuild`
  - Opérations de formatage de disques
- Les partitions doivent avoir des UUID pour être gérées
- Émulateur de terminal (pour les opérations de formatage) : kgx, gnome-terminal, konsole, xfce4-terminal, alacritty, kitty, ou xterm

## Architecture

L'application est structurée en trois couches principales :

1. **Models** (`src/models/`) : Structures de données pour les disques et partitions
2. **Utils** (`src/utils/`) : Analyse et écriture de la configuration NixOS, détection des disques
3. **UI** (`src/ui/`) : Interface GTK4/Libadwaita avec widgets et dialogues

### Composants clés

- **Analyseur de disques** : Lit `/proc/partitions`, utilise `lsblk` et `blkid` pour les informations sur les disques
- **Formateur de disques** : Crée des tables de partition GPT et des systèmes de fichiers ext4 avec configuration automatique des permissions
- **Générateur de configuration** : Génère la configuration NixOS des systèmes de fichiers avec les options de montage appropriées
- **Détection des manquants** : Compare le matériel configuré et réel pour avertir des disques supprimés
- **Détection de la locale** : Lit les variables d'environnement `LANG` et `LC_ALL`
- **Fonctionnalités de sécurité** :
  - Filtre les partitions système critiques (/, /boot, /nix)
  - Dialogues de confirmation pour les opérations destructives
  - Configuration automatique des permissions pour la propriété utilisateur

## Développement

Le projet utilise :
- **Cargo** pour la compilation Rust et la gestion des dépendances
- **Meson** pour l'intégration avec le système de build Nix
- **gettext** pour l'internationalisation (français et anglais actuellement supportés)

## Licence

GPL-3.0-or-later

## Migration depuis la version Python

L'implémentation Python originale n'est plus maintenue. Différences clés :

- L'interface est maintenant construite programmatiquement (pas de fichiers Blueprint dans la version Rust)
- Meilleure gestion des erreurs avec les types Result de Rust
- Interface plus réactive avec des opérations asynchrones
- Binaire plus petit et démarrage plus rapide

Toutes les fonctionnalités originales sont préservées et améliorées.

---

# Nix-disk

A modern GTK4/Libadwaita GUI application for managing disk mount configurations on NixOS.

## Version 2.1 - Disk Formatting Support

This version adds disk formatting capabilities and fixes permission issues.

### What's New in 2.1

- **Disk Formatting**: Format disks directly from the application
  - Creates GPT partition table
  - Formats with ext4 filesystem
  - Custom volume labels
  - Automatic permission setup for the current user
- **Permission Fix**: Newly formatted disks are now automatically configured with correct ownership
  - No more "Permission denied" errors after formatting
  - Users can immediately write to their formatted disks

### Version 2.0 - Rust Rewrite

Complete rewrite in Rust for better performance, memory safety, and reliability:

- **Automatic Language Detection**: Detects and uses your system's locale automatically
- **Missing Partition Detection**: Checks for configured partitions that no longer exist at startup
  - Shows a dialog with the list of missing partitions
  - Option to automatically remove them from configuration

### Core Features

- Visual management of disk mount points in NixOS
- Format disks with ext4 filesystem
- Reads and modifies `/etc/nixos/hardware-configuration.nix`
- Automatic filesystem-specific mount options (btrfs compression, NTFS permissions, etc.)
- Live rebuild with `nixos-rebuild switch`
- Status banners for rebuild progress and errors
- Support for multiple mount points per partition
- Default mount point: `/media/` (user-friendly location)

## Building

### Using Nix (Recommended)

```bash
# Build the package
nix build

# Run directly
nix run

# Enter development environment
nix develop
```

### Using Cargo

```bash
# Enter development environment first
nix develop

# Build
cargo build --release

# Run
cargo run --release
```

## Requirements

- NixOS with `/etc/nixos/hardware-configuration.nix`
- Root privileges (via sudo/polkit) for:
  - `nixos-rebuild` operations
  - Disk formatting operations
- Partitions must have UUIDs to be managed
- Terminal emulator (for formatting operations): kgx, gnome-terminal, konsole, xfce4-terminal, alacritty, kitty, or xterm

## Architecture

The application is structured in three main layers:

1. **Models** (`src/models/`): Data structures for disks and partitions
2. **Utils** (`src/utils/`): Parsing and writing NixOS configuration, disk detection
3. **UI** (`src/ui/`): GTK4/Libadwaita interface with widgets and dialogs

### Key Components

- **Disk Parser**: Reads `/proc/partitions`, uses `lsblk` and `blkid` for disk information
- **Disk Formatter**: Creates GPT partition tables and ext4 filesystems with automatic permission setup
- **Config Writer**: Generates NixOS filesystem configuration with appropriate mount options
- **Missing Detection**: Compares configured vs. actual hardware to warn about removed disks
- **Locale Detection**: Reads `LANG` and `LC_ALL` environment variables
- **Safety Features**:
  - Filters out critical system partitions (/, /boot, /nix)
  - Confirmation dialogs for destructive operations
  - Automatic permission setup for user ownership

## Development

The project uses:
- **Cargo** for Rust compilation and dependency management
- **Meson** for integration with Nix build system
- **gettext** for internationalization (French and English currently supported)

## License

GPL-3.0-or-later

## Migration from Python Version

The original Python implementation is no longer maintained. Key differences:

- UI is now constructed programmatically (no Blueprint files in Rust version)
- Better error handling with Rust's Result types
- More responsive UI with async operations
- Smaller binary size and faster startup

All original functionality is preserved and enhanced.
