use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Partition {
    pub path: PathBuf,
    pub uuid_path: PathBuf,
    pub mount_points: Vec<String>,
    pub fs_type: Option<String>,
    pub size: u64,
    pub label: Option<String>,
}

impl Partition {
    pub fn new(
        path: PathBuf,
        uuid_path: PathBuf,
        mount_points: Vec<String>,
        fs_type: Option<String>,
        size: u64,
        label: Option<String>,
    ) -> Self {
        Self {
            path,
            uuid_path,
            mount_points,
            fs_type,
            size,
            label,
        }
    }

    pub fn add_mount_point(&mut self, mount_point: String) {
        if !self.mount_points.contains(&mount_point) {
            self.mount_points.push(mount_point);
        }
    }

    #[allow(dead_code)]
    pub fn remove_mount_point(&mut self, mount_point: &str) {
        self.mount_points.retain(|mp| mp != mount_point);
    }
}
