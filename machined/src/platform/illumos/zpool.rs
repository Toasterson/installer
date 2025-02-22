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
    for vdev in &pool.vdevs {
        zpool_cmd.arg(vdev.kind.to_string().as_str());
        zpool_cmd.args(vdev.disks.iter().map(|d| d.as_str()));
    }
    let out = zpool_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::ZpoolCreateFailed(String::from_utf8(
            out.stderr,
        )?));
    }
    Ok(())
}

pub fn create_boot_environment_base_dataset() -> Result<(), InstallationError> {
    let out = Command::new(ZFS_BIN)
        .args(["create", "rpool/ROOT"])
        .output()?;
    if !out.status.success() {
        return Err(InstallationError::BaseRootDSCreateFailed(
            String::from_utf8(out.stderr)?,
        ));
    }
    Ok(())
}

fn generate_be_name() -> String {
    let now = chrono::Utc::now();
    format!("openindiana-{}", now.format("%Y-%m-%d:%H:%M").to_string())
}

pub fn create_boot_environment(be_name: Option<String>) -> Result<String, InstallationError> {
    let mut zfs_cmd = Command::new(ZFS_BIN).arg("create");

    let be_name = if let Some(be_name) = be_name {
        be_name.clone()
    } else {
        generate_be_name()
    };

    let boot_env = format!("rpool/ROOT/{}", be_name);

    zfs_cmd.arg(&boot_env);
    let out = zfs_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::BaseRootDSCreateFailed(
            String::from_utf8(out.stderr)?,
        ));
    }

    Ok(boot_env)
}

pub fn mount_boot_environment(be_path: String) -> Result<(), InstallationError> {
    let mut zfs_cmd = Command::new(ZFS_BIN).args(["mount", &be_path, "/a"]);

    let out = zfs_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::BaseRootDSCreateFailed(
            String::from_utf8(out.stderr)?,
        ));
    }
    Ok(())
}
