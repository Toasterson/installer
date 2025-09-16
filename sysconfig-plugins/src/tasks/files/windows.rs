use std::fs;
use std::io::{self, Write};
use std::path::Path;

use super::{Ensure, FileSpec};

pub fn diff(specs: &[FileSpec]) -> io::Result<Vec<crate::TaskChange>> {
    let mut changes = Vec::new();
    for s in specs {
        match s.ensure {
            Ensure::Absent => {
                if Path::new(&s.path).exists() {
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
                        })),
                        verbose: s.content.is_some(),
                    });
                } else if let Some(desired) = &s.content {
                    if let Ok(cur) = fs::read_to_string(p) {
                        if cur != *desired {
                            changes.push(crate::TaskChange {
                            change_type: crate::TaskChangeType::Update,
                            path: s.path.clone(),
                            old_value: Some(serde_json::json!({"content": cur})),
                            new_value: Some(serde_json::json!({"content": desired})),
                            verbose: true,
                        });
                        }
                    } else {
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
        }
    }
    Ok(changes)
}

pub fn apply(specs: &[FileSpec], dry_run: bool) -> io::Result<Vec<crate::TaskChange>> {
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
                    if !dry_run {
                        let _ = fs::remove_file(p);
                    }
                }
            }
            Ensure::Present => {
                let p = Path::new(&s.path);
                if let Some(parent) = p.parent() {
                    if !dry_run {
                        fs::create_dir_all(parent)?;
                    }
                }
                if let Some(content) = &s.content {
                    let write_needed = match fs::read_to_string(p) {
                        Ok(cur) => cur != *content,
                        Err(_) => true,
                    };
                    if write_needed {
                        changes.push(crate::TaskChange {
                            change_type: if p.exists() { crate::TaskChangeType::Update } else { crate::TaskChangeType::Create },
                            path: s.path.clone(),
                            old_value: None,
                            new_value: Some(serde_json::json!({"content": content})),
                            verbose: true,
                        });
                        if !dry_run {
                            atomic_write(p, content.as_bytes())?;
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
                    if !dry_run {
                        atomic_write(p, b"")?;
                    }
                }
                // Ignore mode/uid/gid on Windows for now (no ACLs)
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
