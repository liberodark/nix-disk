use crate::models::Disk;
use anyhow::Result;

/// List of critical mount points that should NEVER be removed
const CRITICAL_MOUNT_POINTS: &[&str] = &["/", "/boot", "/boot/efi", "/nix", "/nix/store"];

/// Check if a mount point is critical for system boot/operation
fn is_critical_mount_point(mount_point: &str) -> bool {
    CRITICAL_MOUNT_POINTS.contains(&mount_point)
}

/// Extract mount point from a fileSystems block
fn extract_mount_point(block: &str) -> Option<String> {
    if let Some(start) = block.find("fileSystems.\"") {
        let after_start = &block[start + 13..]; // Skip 'fileSystems."'
        if let Some(end) = after_start.find('"') {
            return Some(after_start[..end].to_string());
        }
    }
    None
}

/// Generate NixOS disk configuration from disk list
pub fn get_nix_disks_config(nix_config: &str, disks: &[Disk]) -> Result<String> {
    eprintln!("🔧 get_nix_disks_config appelé");
    eprintln!("🔧 Nombre de disques à traiter: {}", disks.len());

    let mut config = nix_config.to_string();
    let mut preserved_blocks = Vec::new();

    // First pass: extract and preserve critical fileSystems blocks
    let mut pos = 0;
    while let Some(found_pos) = config[pos..].find("fileSystems.\"") {
        let absolute_pos = pos + found_pos;
        if let Some(close_pos) = config[absolute_pos..].find("};") {
            let block = &config[absolute_pos..absolute_pos + close_pos + 2];
            eprintln!("🔍 Bloc trouvé:\n{}", block);

            if let Some(mount_point) = extract_mount_point(block) {
                eprintln!("🔍 Mount point extrait: {}", mount_point);
                if is_critical_mount_point(&mount_point) {
                    eprintln!("🔒 Préservation du filesystem critique: {}", mount_point);
                    preserved_blocks.push(block.to_string());
                } else {
                    eprintln!("ℹ️  Mount point non critique: {}", mount_point);
                }
            } else {
                eprintln!("⚠️  Impossible d'extraire le mount point du bloc");
            }

            pos = absolute_pos + close_pos + 2;
        } else {
            break;
        }
    }

    // Remove all existing fileSystems blocks (including critical ones temporarily)
    let mut removed_count = 0;
    while let Some(pos) = config.find("fileSystems.\"") {
        if let Some(close_pos) = config[pos..].find("};") {
            config.replace_range(pos..pos + close_pos + 2, "");
            removed_count += 1;
        } else {
            break;
        }
    }
    eprintln!("🔧 Blocs fileSystems supprimés: {}", removed_count);
    eprintln!("🔒 Blocs critiques préservés: {}", preserved_blocks.len());

    // Remove trailing newlines before the closing brace
    config = config.trim_end().to_string();
    if config.ends_with('}') {
        config.pop();
    }

    // First, restore preserved critical fileSystems blocks
    for preserved in &preserved_blocks {
        config.push_str("\n  ");
        config.push_str(preserved);
    }

    // Then generate new fileSystems blocks for non-critical mount points
    let mut generated_count = 0;
    for (disk_idx, disk) in disks.iter().enumerate() {
        eprintln!("🔧 Traitement disque {}: {}", disk_idx, disk.path.display());
        eprintln!("🔧   Nombre de partitions: {}", disk.partitions.len());

        for (part_idx, partition) in disk.partitions.iter().enumerate() {
            eprintln!("🔧   Partition {}: {}", part_idx, partition.path.display());
            eprintln!("🔧     Points de montage: {:?}", partition.mount_points);
            eprintln!("🔧     Type FS: {:?}", partition.fs_type);

            if partition.mount_points.is_empty() {
                eprintln!("🔧     → Ignorée (pas de points de montage)");
                continue;
            }

            let fs_type = partition.fs_type.as_deref().unwrap_or("auto");

            for mount_point in &partition.mount_points {
                // Skip if this is a critical mount point (already preserved)
                if is_critical_mount_point(mount_point) {
                    eprintln!("🔧     → Ignorée (point de montage critique déjà préservé): {}", mount_point);
                    continue;
                }

                let fs_options = get_filesystem_options(fs_type, mount_point);

                eprintln!("🔧     → Génération entrée pour: {}", mount_point);
                config.push_str("\n  fileSystems.\"");
                config.push_str(mount_point);
                config.push_str("\" = {\n");
                config.push_str("    device = \"");
                config.push_str(&partition.uuid_path.display().to_string());
                config.push_str("\";\n");
                config.push_str("    fsType = \"");
                config.push_str(fs_type);
                config.push_str("\";\n");
                config.push_str("    options = ");
                config.push_str(&fs_options);
                config.push_str(";\n  };");
                generated_count += 1;
            }
        }
    }

    config.push_str("\n\n}");

    eprintln!("🔧 Total d'entrées fileSystems générées: {}", generated_count);
    eprintln!("🔧 Taille de la config générée: {} octets", config.len());

    Ok(config)
}

/// Get filesystem-specific mount options
fn get_filesystem_options(fs_type: &str, mount_point: &str) -> String {
    let mut options = match fs_type {
        "btrfs" => vec![
            "defaults",
            "nofail",
            "x-gvfs-show",
            "compress=zstd",
        ],
        "ntfs" | "ntfs3" => vec![
            "defaults",
            "nofail",
            "x-gvfs-show",
            "uid=1000",
            "gid=100",
            "umask=022",
        ],
        _ => vec!["defaults", "nofail", "x-gvfs-show"],
    };

    // Remove x-gvfs-show for root and boot partitions
    if mount_point == "/" || mount_point == "/boot" {
        options.retain(|&opt| opt != "x-gvfs-show");
    }

    format!("[ {} ]", options.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_options() {
        let btrfs_opts = get_filesystem_options("btrfs", "/media/data");
        assert!(btrfs_opts.contains("compress=zstd"));
        assert!(btrfs_opts.contains("x-gvfs-show"));

        let root_opts = get_filesystem_options("ext4", "/");
        assert!(!root_opts.contains("x-gvfs-show"));
    }
}
