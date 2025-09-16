// Minimal stub for Windows base plugin. This is intentionally lightweight and
// does not start a server. For now, only a subset of plugins will run on
// Windows; this binary serves as a placeholder.

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("windows-base-plugin is stubbed and intended for Windows only. Nothing to do on this OS.");
}

#[cfg(target_os = "windows")]
fn main() {
    println!("windows-base-plugin stub: Windows support is planned; only a subset of tasks will be implemented. Exiting.");
}
