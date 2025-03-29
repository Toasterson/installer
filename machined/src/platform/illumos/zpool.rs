use crate::error::InstallationError;
use machineconfig::Pool;
use std::process::Command;
use uuid::Uuid;

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
    create_dataset(
        "rpool/ROOT",
        false,
        Some(&[("canmount", "off"), ("mountpoint", "legacy")]),
    )
}

fn generate_be_name() -> String {
    let now = chrono::Utc::now();
    format!("openindiana-{}", now.format("%Y-%m-%d:%H:%M").to_string())
}

pub fn create_dataset<S>(
    name: &str,
    parents: bool,
    opts: Option<&[(S, S)]>,
) -> Result<(), InstallationError>
where
    S: AsRef<str> + std::fmt::Display,
{
    let mut zfs_cmd = Command::new(ZFS_BIN);
    zfs_cmd.arg("create");
    if parents {
        zfs_cmd.arg("-p");
    }
    if let Some(opts) = opts {
        for (key, value) in opts {
            zfs_cmd.args(&["-o", format!("{}={}", key, value).as_str()]);
        }
    }
    zfs_cmd.arg(name);
    let out = zfs_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::BaseRootDSCreateFailed(
            String::from_utf8(out.stderr)?,
        ));
    }

    Ok(())
}

pub fn set_dataset_property(name: &str, value: &str) -> Result<(), InstallationError> {
    let pair = format!("{}={}", name, value);
    let mut zfs_cmd = Command::new(ZFS_BIN);
    zfs_cmd.arg("set");
    zfs_cmd.arg(pair.as_str());
    let out = zfs_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::ZfsSetFailed(String::from_utf8(
            out.stderr,
        )?));
    }

    Ok(())
}

pub fn create_boot_environment(be_name: Option<String>) -> Result<String, InstallationError> {
    let be_name = if let Some(be_name) = be_name {
        be_name.clone()
    } else {
        generate_be_name()
    };

    let uuid = Uuid::new_v4().as_hyphenated().to_string();

    let boot_env = format!("rpool/ROOT/{}", be_name);
    create_dataset(
        be_name.as_str(),
        true,
        Some(&[
            ("canmount", "noauto"),
            ("mountpoint", "legacy"),
            ("org.opensolaris.libbe:uuid", &uuid),
            ("org.opensolaris.libbe:policy", "static"),
        ]),
    )?;
    Ok(boot_env)
}

pub fn mount_boot_environment(be_path: &str) -> Result<(), InstallationError> {
    let mut zfs_cmd = Command::new(ZFS_BIN);
    zfs_cmd.args(["mount", &be_path, "/a"]);

    let out = zfs_cmd.output()?;
    if !out.status.success() {
        return Err(InstallationError::BaseRootDSCreateFailed(
            String::from_utf8(out.stderr)?,
        ));
    }
    Ok(())
}
