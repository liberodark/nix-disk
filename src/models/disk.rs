use super::partition::Partition;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Disk {
    pub path: PathBuf,
    pub partitions: Vec<Partition>,
    pub size: u64,
}

impl Disk {
    pub fn new(path: PathBuf, partitions: Vec<Partition>, size: u64) -> Self {
        Self {
            path,
            partitions,
            size,
        }
    }

    pub fn add_partition(&mut self, partition: Partition) {
        self.partitions.push(partition);
    }
}
