use std::io;

// Windows is currently not supported for these tasks in this project; stub out.
pub fn apply_hostname(_hostname: &str, _dry_run: bool) -> io::Result<bool> {
    Ok(false)
}

pub fn apply_dns(_nameservers: &[String], _search: &[String], _dry_run: bool) -> io::Result<bool> {
    Ok(false)
}
