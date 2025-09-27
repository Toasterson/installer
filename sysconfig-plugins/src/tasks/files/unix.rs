use std::fs;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use tracing::{debug, info};

use super::{Ensure, FileSpec};

pub fn diff(specs: &[FileSpec]) -> io::Result<Vec<crate::TaskChange>> {
    let mut changes = Vec::new();
    for s in specs {
        match s.ensure {
            Ensure::Absent => {
                let p = Path::new(&s.path);
                if p.exists() {
                    changes.push(crate::TaskChange {
                        change_type: crate::TaskChangeType::Delete,
                        path: s.path.clone(),
                        old_value: Some(serde_json::json!({"exists": true})),
                        new_value: None,
                        verbose: false,
                    });
                }
            }
            Ensure::Present => {
                let p = Path::new(&s.path);
                if !p.exists() {
                    changes.push(crate::TaskChange {
                        change_type: crate::TaskChangeType::Create,
                        path: s.path.clone(),
                        old_value: None,
                        new_value: Some(serde_json::json!({
                            "content": s.content.clone().unwrap_or_default(),
                            "mode": s.mode.clone(),
                            "uid": s.uid,
                            "gid": s.gid,
                        })),
                        verbose: s.content.is_some(),
                    });
                } else {
                    if let Some(desired) = &s.content {
                        match fs::read_to_string(p) {
                            Ok(cur) => {
                                if cur != *desired {
                                    changes.push(crate::TaskChange {
                                        change_type: crate::TaskChangeType::Update,
                                        path: s.path.clone(),
                                        old_value: Some(serde_json::json!({"content": cur})),
                                        new_value: Some(serde_json::json!({"content": desired})),
                                        verbose: true,
                                    });
                                }
                            }
                            Err(_) => {
                                changes.push(crate::TaskChange {
                                    change_type: crate::TaskChangeType::Update,
                                    path: s.path.clone(),
                                    old_value: None,
                                    new_value: Some(serde_json::json!({"content": desired})),
                                    verbose: true,
                                });
                            }
                        }
                    }
                    if let Some(m) = &s.mode {
                        if let Ok(cur) = current_mode(p) {
                            if parse_mode(m).unwrap_or(cur) != cur {
                                changes.push(crate::TaskChange {
                                    change_type: crate::TaskChangeType::Update,
                                    path: s.path.clone(),
                                    old_value: Some(
                                        serde_json::json!({"mode": format!("{:o}", cur)}),
                                    ),
                                    new_value: Some(serde_json::json!({"mode": m})),
                                    verbose: false,
                                });
                            }
                        } else {
                            changes.push(crate::TaskChange {
                                change_type: crate::TaskChangeType::Update,
                                path: s.path.clone(),
                                old_value: None,
                                new_value: Some(serde_json::json!({"mode": m})),
                                verbose: false,
                            });
                        }
                    }
                    if s.uid.is_some() || s.gid.is_some() {
                        let (old_uid, old_gid) = match fs::metadata(p) {
                            Ok(md) => (md.uid(), md.gid()),
                            Err(_) => (u32::MAX, u32::MAX),
                        };
                        changes.push(crate::TaskChange {
                            change_type: crate::TaskChangeType::Update,
                            path: s.path.clone(),
                            old_value: Some(serde_json::json!({"uid": old_uid, "gid": old_gid})),
                            new_value: Some(serde_json::json!({"uid": s.uid, "gid": s.gid})),
                            verbose: false,
                        });
                    }
                }
            }
        }
    }
    Ok(changes)
}

pub fn apply(specs: &[FileSpec], dry_run: bool) -> io::Result<Vec<crate::TaskChange>> {
    if dry_run {
        info!("DRY-RUN: Checking file configurations...");
    }
    let mut changes = Vec::new();
    for s in specs {
        match s.ensure {
            Ensure::Absent => {
                let p = Path::new(&s.path);
                if p.exists() {
                    changes.push(crate::TaskChange {
                        change_type: crate::TaskChangeType::Delete,
                        path: s.path.clone(),
                        old_value: Some(serde_json::json!({"exists": true})),
                        new_value: None,
                        verbose: false,
                    });
                    if dry_run {
                        info!("DRY-RUN: Would delete file: {}", s.path);
                    } else {
                        let _ = fs::remove_file(p);
                        info!("Deleted file: {}", s.path);
                    }
                }
                continue;
            }
            Ensure::Present => {
                let p = Path::new(&s.path);
                if let Some(parent) = p.parent() {
                    if dry_run {
                        if !parent.exists() {
                            info!(
                                "DRY-RUN: Would create parent directories for: {}",
                                parent.display()
                            );
                        }
                    } else {
                        fs::create_dir_all(parent)?;
                    }
                }
                // Content
                if let Some(content) = &s.content {
                    let write_needed = match fs::read_to_string(p) {
                        Ok(cur) => cur != *content,
                        Err(_) => true,
                    };
                    if write_needed {
                        let action = if p.exists() { "update" } else { "create" };
                        changes.push(crate::TaskChange {
                            change_type: if p.exists() {
                                crate::TaskChangeType::Update
                            } else {
                                crate::TaskChangeType::Create
                            },
                            path: s.path.clone(),
                            old_value: None,
                            new_value: Some(serde_json::json!({"content": content})),
                            verbose: true,
                        });
                        if dry_run {
                            info!("DRY-RUN: Would {} file: {}", action, s.path);
                            if content.len() <= 100 {
                                debug!("DRY-RUN:   Content: {:?}", content);
                            } else {
                                debug!("DRY-RUN:   Content: {} bytes (truncated)", content.len());
                            }
                        } else {
                            atomic_write(p, content.as_bytes())?;
                            info!(
                                "File {} {}: {}",
                                s.path,
                                action,
                                if content.len() <= 100 {
                                    format!("{} bytes", content.len())
                                } else {
                                    format!("{} bytes", content.len())
                                }
                            );
                        }
                    }
                } else if !p.exists() {
                    changes.push(crate::TaskChange {
                        change_type: crate::TaskChangeType::Create,
                        path: s.path.clone(),
                        old_value: None,
                        new_value: Some(serde_json::json!({"content": ""})),
                        verbose: false,
                    });
                    if dry_run {
                        info!("DRY-RUN: Would create empty file: {}", s.path);
                    } else {
                        atomic_write(p, b"")?;
                        info!("Created empty file: {}", s.path);
                    }
                }
                // Mode
                if let Some(m) = &s.mode {
                    if let Some(mode) = parse_mode(m) {
                        let cur = current_mode(p).unwrap_or(0);
                        if cur != mode {
                            changes.push(crate::TaskChange {
                                change_type: crate::TaskChangeType::Update,
                                path: s.path.clone(),
                                old_value: Some(serde_json::json!({"mode": format!("{:o}", cur)})),
                                new_value: Some(serde_json::json!({"mode": format!("{:o}", mode)})),
                                verbose: false,
                            });
                            if dry_run {
                                info!(
                                    "DRY-RUN: Would change permissions for {} from {:o} to {:o}",
                                    s.path, cur, mode
                                );
                            } else {
                                fs::set_permissions(p, fs::Permissions::from_mode(mode))?;
                                info!("Changed permissions for {} to {:o}", s.path, mode);
                            }
                        }
                    }
                }
                // Ownership
                if s.uid.is_some() || s.gid.is_some() {
                    let uid = s.uid.unwrap_or(u32::MAX); // MAX => keep current
                    let gid = s.gid.unwrap_or(u32::MAX);
                    let (old_uid, old_gid) = match fs::metadata(p) {
                        Ok(md) => (md.uid(), md.gid()),
                        Err(_) => (u32::MAX, u32::MAX),
                    };
                    changes.push(crate::TaskChange {
                        change_type: crate::TaskChangeType::Update,
                        path: s.path.clone(),
                        old_value: Some(serde_json::json!({"uid": old_uid, "gid": old_gid})),
                        new_value: Some(serde_json::json!({"uid": s.uid, "gid": s.gid})),
                        verbose: false,
                    });
                    if dry_run {
                        let new_uid_str = if uid == u32::MAX {
                            "unchanged".to_string()
                        } else {
                            uid.to_string()
                        };
                        let new_gid_str = if gid == u32::MAX {
                            "unchanged".to_string()
                        } else {
                            gid.to_string()
                        };
                        info!(
                            "DRY-RUN: Would change ownership for {} from {}:{} to {}:{}",
                            s.path, old_uid, old_gid, new_uid_str, new_gid_str
                        );
                    } else {
                        chown_path(p, uid, gid)?;
                        info!(
                            "Changed ownership for {} to uid:{} gid:{}",
                            s.path,
                            if uid == u32::MAX { old_uid } else { uid },
                            if gid == u32::MAX { old_gid } else { gid }
                        );
                    }
                }
            }
        }
    }
    Ok(changes)
}

fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

fn parse_mode(s: &str) -> Option<u32> {
    let t = s.trim_start_matches("0o").trim_start_matches('0');
    u32::from_str_radix(if t.is_empty() { "0" } else { t }, 8).ok()
}

fn current_mode(path: &Path) -> io::Result<u32> {
    Ok(fs::metadata(path)?.permissions().mode())
}

fn chown_path(path: &Path, uid: u32, gid: u32) -> io::Result<()> {
    // u32::MAX means keep current
    let md = fs::metadata(path)?;
    let cur_uid = md.uid();
    let cur_gid = md.gid();
    let new_uid = if uid == u32::MAX { cur_uid } else { uid };
    let new_gid = if gid == u32::MAX { cur_gid } else { gid };
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    let rc = unsafe {
        libc::chown(
            c_path.as_ptr(),
            new_uid as libc::uid_t,
            new_gid as libc::gid_t,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
