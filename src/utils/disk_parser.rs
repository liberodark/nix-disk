use crate::models::{Disk, Partition};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Parse NixOS filesystem configurations from hardware-configuration.nix
pub fn parse_nix_filesystems(nix_config: &str) -> Result<HashMap<PathBuf, Partition>> {
    let mut partitions = HashMap::new();

    let fs_regex = Regex::new(r#"fileSystems\."(.+?)""#)?;
    let device_regex = Regex::new(r#"device = "(.+?)";"#)?;

    for fs_match in fs_regex.find_iter(nix_config) {
        let start = fs_match.start();
        let end = nix_config[start..].find('}').unwrap_or(nix_config.len()) + start + 1;
        let nix_group = &nix_config[start..end];

        // Extract mount point
        let mount_point = fs_regex
            .captures(nix_group)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .context("Failed to extract mount point")?;

        // Extract device (UUID path)
        let partition_uuid_path = device_regex
            .captures(nix_group)
            .and_then(|cap| cap.get(1))
            .map(|m| PathBuf::from(m.as_str()))
            .context("Failed to extract device path")?;

        // Resolve symlink to get actual partition path
        let partition_path = if partition_uuid_path.exists() {
            fs::read_link(&partition_uuid_path)
                .map(|link| {
                    let parent = partition_uuid_path.parent().unwrap_or(Path::new("/dev/disk/by-uuid"));
                    parent.join(&link).canonicalize().unwrap_or_else(|_| link)
                })
                .unwrap_or_else(|_| partition_uuid_path.clone())
        } else {
            partition_uuid_path.clone()
        };

        // Add or update partition
        partitions
            .entry(partition_path.clone())
            .and_modify(|p: &mut Partition| p.add_mount_point(mount_point.clone()))
            .or_insert_with(|| {
                Partition::new(
                    partition_path,
                    partition_uuid_path,
                    vec![mount_point],
                    None,
                    0,
                    None,
                )
            });
    }

    Ok(partitions)
}

/// Get all disks from the system
pub fn get_disks(nix_config: Option<&str>) -> Result<Vec<Disk>> {
    let mut partitions = if let Some(config) = nix_config {
        parse_nix_filesystems(config)?
    } else {
        HashMap::new()
    };

    // Read /proc/partitions
    let proc_partitions = fs::read_to_string("/proc/partitions")
        .context("Failed to read /proc/partitions")?;

    let mut disks: Vec<Disk> = Vec::new();

    for line in proc_partitions.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let disk_name = parts[3];
        let disk_path = PathBuf::from(format!("/dev/{}", disk_name));

        // Skip zram and CD/DVD devices
        if disk_name.starts_with("zram") || disk_name.starts_with("sr") {
            continue;
        }

        // Get disk size using lsblk
        let size_output = Command::new("lsblk")
            .args(&["-b", "--output", "SIZE", "-n", "-d"])
            .arg(&disk_path)
            .output()
            .context("Failed to run lsblk")?;

        let disk_size = String::from_utf8_lossy(&size_output.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        // Check if this is a partition of ANY existing disk
        let mut is_partition = false;
        for disk in disks.iter_mut() {
            // A partition path starts with its parent disk path
            // e.g., /dev/sda1 starts with /dev/sda
            //       /dev/nvme0n1p1 starts with /dev/nvme0n1
            let disk_path_str = disk_path.to_string_lossy();
            let parent_path_str = disk.path.to_string_lossy();

            if disk_path_str.starts_with(parent_path_str.as_ref()) && disk_path != disk.path {
                // This is a partition of this disk
                let partition = parse_partition(&disk_path, disk_size, &mut partitions)?;
                if let Some(part) = partition {
                    disk.add_partition(part);
                }
                is_partition = true;
                break;
            }
        }

        if is_partition {
            continue;
        }

        // This is a new disk
        disks.push(Disk::new(disk_path, Vec::new(), disk_size));
    }

    Ok(disks)
}

/// Parse a single partition
fn parse_partition(
    partition_path: &Path,
    size: u64,
    partitions_map: &mut HashMap<PathBuf, Partition>,
) -> Result<Option<Partition>> {
    // Get partition info using blkid
    let blkid_output = Command::new("blkid")
        .arg(partition_path)
        .output()
        .context("Failed to run blkid")?;

    let blkid_str = String::from_utf8_lossy(&blkid_output.stdout);

    // Extract filesystem type
    let type_regex = Regex::new(r#"TYPE="([^"]+)""#)?;
    let fs_type = type_regex
        .captures(&blkid_str)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string());

    // Extract label
    let label_regex = Regex::new(r#"LABEL="([^"]+)""#)?;
    let label = label_regex
        .captures(&blkid_str)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string());

    // Check if this partition is already in our config
    if let Some(existing_partition) = partitions_map.get_mut(partition_path) {
        existing_partition.fs_type = fs_type;
        existing_partition.label = label;
        existing_partition.size = size;
        return Ok(Some(existing_partition.clone()));
    }

    // Extract UUID
    let uuid_regex = Regex::new(r#"UUID="([^"]+)""#)?;
    if let Some(uuid_cap) = uuid_regex.captures(&blkid_str) {
        if let Some(uuid) = uuid_cap.get(1) {
            let uuid_path = PathBuf::from(format!("/dev/disk/by-uuid/{}", uuid.as_str()));
            return Ok(Some(Partition::new(
                partition_path.to_path_buf(),
                uuid_path,
                Vec::new(),
                fs_type,
                size,
                label,
            )));
        }
    }

    // No UUID found, skip this partition
    Ok(None)
}

/// Compare configured partitions with existing ones and find missing partitions
pub fn find_missing_partitions(
    configured_partitions: &[Partition],
    existing_disks: &[Disk],
) -> Vec<Partition> {
    // Collect all existing partition UUIDs - this is the most reliable way
    let existing_uuids: std::collections::HashSet<PathBuf> = existing_disks
        .iter()
        .flat_map(|disk| disk.partitions.iter().map(|p| p.uuid_path.clone()))
        .collect();

    // A partition is missing if its UUID path is not in the existing UUIDs
    configured_partitions
        .iter()
        .filter(|p| !existing_uuids.contains(&p.uuid_path))
        .cloned()
        .collect()
}
