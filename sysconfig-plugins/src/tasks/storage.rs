//! Storage-related management for base plugins.
//!
//! Provides platform-specific storage implementations:
//! - illumos: ZFS (pools, datasets, volumes)
//! - FreeBSD: ZFS and UFS
//! - Linux: ext4, xfs, btrfs, LVM

use serde_json::Value;

#[derive(Default)]
pub struct StorageZfs;

impl crate::TaskHandler for StorageZfs {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired ZFS state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply ZFS changes (create pools/datasets/volumes, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative ZFS actions (e.g. snapshot, rollback)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct StorageUfs;

impl crate::TaskHandler for StorageUfs {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired UFS state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply UFS changes (create filesystems, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative UFS actions (e.g. fsck, dump)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct StorageExt4;

impl crate::TaskHandler for StorageExt4 {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired ext4 state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply ext4 changes (create filesystems, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative ext4 actions (e.g. fsck, resize2fs)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct StorageXfs;

impl crate::TaskHandler for StorageXfs {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired XFS state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply XFS changes (create filesystems, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative XFS actions (e.g. xfs_repair, xfs_growfs)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct StorageBtrfs;

impl crate::TaskHandler for StorageBtrfs {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired Btrfs state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply Btrfs changes (create subvolumes, snapshots, set properties)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative Btrfs actions (e.g. snapshot, balance)
        Ok(String::new())
    }
}

#[derive(Default)]
pub struct StorageLvm;

impl crate::TaskHandler for StorageLvm {
    fn diff(&self, _current: &Value, _desired: &Value) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: compute differences between current and desired LVM state
        Ok(Vec::new())
    }

    fn apply(&self, _desired: &Value, _dry_run: bool) -> Result<Vec<crate::TaskChange>, String> {
        // TODO: apply LVM changes (create PV/VG/LV, extend volumes)
        Ok(Vec::new())
    }

    fn exec(&self, _action: &str, _params: &Value) -> Result<String, String> {
        // TODO: run imperative LVM actions (e.g. pvcreate, vgextend, lvextend)
        Ok(String::new())
    }
}
