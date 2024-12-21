use miette::miette;
use crate::process::run_capture_stdout;
use miette::Result;

static DEVPROP_BIN: &str = "/sbin/devprop";
pub fn devprop<S: AsRef<str>>(key: S) -> Result<String> {
    let key = key.as_ref();
    let val = run_capture_stdout(vec![DEVPROP_BIN, key].as_ref(), None)?;
    let lines: Vec<_> = val.lines().collect();
    if lines.len() != 1 {
        Err(miette!("unexpected output for devprop {}: {:?}", key, lines))
    } else {
        Ok(lines[0].trim().to_string())
    }
}