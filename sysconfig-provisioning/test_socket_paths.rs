#!/usr/bin/env rust-script

//! This script tests the socket path logic to ensure it works correctly
//! for both root and non-root users.

fn default_socket_path() -> String {
    #[cfg(target_os = "linux")]
    {
        // Prefer XDG_RUNTIME_DIR if set (usually /run/user/$UID)
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            return format!("{}/sysconfig-provisioning.sock", dir);
        }
        // Fallback to /run/user/$EUID
        let euid = unsafe { libc::geteuid() as u32 };
        if euid == 0 {
            "/var/run/sysconfig-provisioning.sock".to_string()
        } else {
            format!("/run/user/{}/sysconfig-provisioning.sock", euid)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        "/var/run/sysconfig-provisioning.sock".to_string()
    }
}

fn default_service_socket_path() -> String {
    #[cfg(target_os = "linux")]
    {
        // Prefer XDG_RUNTIME_DIR if set (usually /run/user/$UID)
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            return format!("{}/sysconfig.sock", dir);
        }
        // Fallback to /run/user/$EUID
        let euid = unsafe { libc::geteuid() as u32 };
        if euid == 0 {
            "/var/run/sysconfig.sock".to_string()
        } else {
            format!("/run/user/{}/sysconfig.sock", euid)
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        "/var/run/sysconfig.sock".to_string()
    }
}

fn main() {
    let euid = unsafe { libc::geteuid() as u32 };
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR").ok();

    println!("Testing socket path logic:");
    println!("  Current EUID: {}", euid);
    println!("  XDG_RUNTIME_DIR: {:?}", xdg_runtime);
    println!();
    println!("Socket paths:");
    println!("  Provisioning plugin socket: {}", default_socket_path());
    println!("  Sysconfig service socket:   {}", default_service_socket_path());
    println!();

    // Test with different XDG_RUNTIME_DIR values
    println!("Testing with different XDG_RUNTIME_DIR values:");

    std::env::remove_var("XDG_RUNTIME_DIR");
    println!("  Without XDG_RUNTIME_DIR:");
    println!("    Provisioning: {}", default_socket_path());
    println!("    Service:      {}", default_service_socket_path());

    std::env::set_var("XDG_RUNTIME_DIR", "/tmp/test-runtime");
    println!("  With XDG_RUNTIME_DIR=/tmp/test-runtime:");
    println!("    Provisioning: {}", default_socket_path());
    println!("    Service:      {}", default_service_socket_path());
}
