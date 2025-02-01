use crate::error::InstallationError;
use machineconfig::Pool;
use std::process::Command;

const ZPOOL_BIN: &str = "/usr/sbin/zpool";
const ZFS_BIN: &str = "/usr/sbin/zfs";

pub fn create_pool(pool: &Pool) -> Result<(), InstallationError> {
    let mut zpool_cmd = Command::new(ZPOOL_BIN);
    zpool_cmd.args(["create", &pool.name]);
    for opt in &pool.options {
        zpool_cmd.args(["-o", format!("{}={}", opt.name, opt.value).as_str()]);
    }
    for vdev in &pool.vdev {
        zpool_cmd.arg(vdev.kind.as_str());
        zpool_cmd.args(vdev.disks.iter().map(|d| d.name.as_str()));
    }
    let out = zpool_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::ZpoolCreateFailed(
            String::from_utf8_unchecked(out.stderr),
        ));
    }
    Ok(())
}

pub fn create_boot_environment_base_dataset() -> Result<(), InstallationError> {
    let res = Command::new(ZFS_BIN)
        .args(["create", "rpool/ROOT"])
        .output()?;
    if !out.status.success() {
        return Err(InstallationError::BaseRootDSCreateFailed(
            String::from_utf8_unchecked(out.stderr),
        ));
    }
    Ok(())
}
