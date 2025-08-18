use crate::machined::{
    BiosInfo, BaseboardInfo, ChassisInfo, DiskInfo, MemoryArrayInfo, MemoryArrayMappedAddressInfo,
    MemoryDeviceInfo, NetworkInterface, ProcessorInfo, SmbiosInfo, SystemBootInfo, SystemInfo as SmbiosSystemInfo,
    SystemInfoResponse, PartitionInfo,
};
use std::fs;
use std::process::Command;
use std::str::FromStr;
use tonic::Status;
use tracing::{debug, error};

/// Execute the diskinfo command and parse its output
pub fn get_disk_info() -> Result<Vec<DiskInfo>, Status> {
    debug!("Executing diskinfo command");
    
    // Run basic diskinfo command to get general disk information
    let basic_output = Command::new("/usr/bin/diskinfo")
        .output()
        .map_err(|e| {
            error!("Failed to execute diskinfo command: {}", e);
            Status::internal(format!("Failed to execute diskinfo command: {}", e))
        })?;

    if !basic_output.status.success() {
        let error_msg = String::from_utf8_lossy(&basic_output.stderr);
        error!("diskinfo command failed: {}", error_msg);
        return Err(Status::internal(format!("diskinfo command failed: {}", error_msg)));
    }

    // Run diskinfo -P command to get chassis and bay information
    debug!("Executing diskinfo -P command");
    let location_output = Command::new("/usr/bin/diskinfo")
        .arg("-P")
        .output()
        .map_err(|e| {
            error!("Failed to execute diskinfo -P command: {}", e);
            Status::internal(format!("Failed to execute diskinfo -P command: {}", e))
        })?;

    if !location_output.status.success() {
        let error_msg = String::from_utf8_lossy(&location_output.stderr);
        error!("diskinfo -P command failed: {}", error_msg);
        return Err(Status::internal(format!("diskinfo -P command failed: {}", error_msg)));
    }

    // Parse both outputs and merge the results
    let basic_output_str = String::from_utf8_lossy(&basic_output.stdout);
    let location_output_str = String::from_utf8_lossy(&location_output.stdout);
    
    let basic_disks = parse_diskinfo_basic(&basic_output_str)?;
    let location_info = parse_diskinfo_location(&location_output_str)?;
    
    // Merge the information
    merge_disk_info(basic_disks, location_info)
}

/// Parse the output of the basic diskinfo command
fn parse_diskinfo_basic(output: &str) -> Result<Vec<DiskInfo>, Status> {
    let mut disks = Vec::new();
    let mut lines = output.lines();

    // Skip the header line
    if let Some(_header) = lines.next() {
        for line in lines {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 6 {
                error!("Invalid diskinfo output line: {}", line);
                continue;
            }

            let device = fields[1].to_string(); // Skip the TYPE field
            let vendor = fields[2].to_string();
            let product = fields[3].to_string();
            let size_str = fields[4].to_string();
            let removable_str = fields[5].to_string();
            let solid_state_str = fields[6].to_string();

            // Parse size (convert to bytes)
            let size_bytes = parse_size(&size_str).unwrap_or(0);

            // Parse removable flag
            let removable = removable_str.eq_ignore_ascii_case("yes");

            // Parse solid state flag
            let solid_state = solid_state_str.eq_ignore_ascii_case("yes");

            // Get all paths to this disk
            let paths = get_disk_paths(&device).unwrap_or_else(|_| vec![]);

            let disk_info = DiskInfo {
                device,
                vendor,
                product,
                serial: String::new(), // Will be filled in from location info
                size_bytes,
                removable,
                solid_state,
                paths,
                fault_status: String::new(), // Will be filled in from location info
                location_code: String::new(), // Will be filled in from location info
                chassis_bay: String::new(), // Will be filled in from location info
            };

            disks.push(disk_info);
        }
    }

    Ok(disks)
}

/// Parse the output of the diskinfo -P command to get location information
fn parse_diskinfo_location(output: &str) -> Result<std::collections::HashMap<String, (String, String, String, String)>, Status> {
    let mut location_info = std::collections::HashMap::new();
    let mut lines = output.lines();

    // Skip the header line
    if let Some(_header) = lines.next() {
        for line in lines {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Based on the examples, diskinfo -P output has the following format:
            // DISK                    VID      PID              SERIAL               FLT LOC LOCATION
            // c1t0d0                  Virtio   Block Device     BHYVE-0A7E-C570-228F -   -   -
            // or
            // c1t0d0                  Virtio   Block Device     BHYVE-0A7E-C570-228F -   -   -1,-1
            
            // Split by whitespace, but be careful with the SERIAL field which might contain spaces
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                error!("Invalid diskinfo -P output line: {}", line);
                continue;
            }
            
            let device = parts[0].to_string();
            let vendor = parts[1].to_string();
            
            // The product might contain spaces, so we need to be careful
            // We know that the serial number is the 4th field, so we'll work backwards
            let mut product_parts = Vec::new();
            let mut i = 2;
            while i < parts.len() - 3 {
                product_parts.push(parts[i]);
                i += 1;
            }
            let product = product_parts.join(" ");
            
            let serial = parts[i].to_string();
            let fault_status = parts[i + 1].to_string();
            let location_code = parts[i + 2].to_string();
            
            // The LOCATION field might be missing or might be the last field
            let chassis_bay = if i + 3 < parts.len() {
                parts[i + 3].to_string()
            } else {
                "-".to_string()
            };

            debug!("Parsed location info for {}: serial={}, fault={}, loc={}, chassis_bay={}",
                  device, serial, fault_status, location_code, chassis_bay);

            // Store the location information for this device
            location_info.insert(device, (serial, fault_status, location_code, chassis_bay));
        }
    }

    Ok(location_info)
}

/// Merge the basic disk information with the location information
fn merge_disk_info(
    mut basic_disks: Vec<DiskInfo>,
    location_info: std::collections::HashMap<String, (String, String, String, String)>,
) -> Result<Vec<DiskInfo>, Status> {
    for disk in &mut basic_disks {
        if let Some((serial, fault_status, location_code, chassis_bay)) = location_info.get(&disk.device) {
            disk.serial = serial.clone();
            disk.fault_status = fault_status.clone();
            disk.location_code = location_code.clone();
            disk.chassis_bay = chassis_bay.clone();
        }
    }

    Ok(basic_disks)
}

/// Parse a size string (e.g., "500.0G") to bytes
fn parse_size(size_str: &str) -> Result<u64, Status> {
    let mut size_chars = size_str.chars().peekable();
    let mut size_num_str = String::new();
    let mut unit_str = String::new();

    // Extract the numeric part
    while let Some(c) = size_chars.peek() {
        if c.is_ascii_digit() || *c == '.' {
            size_num_str.push(*c);
            size_chars.next();
        } else {
            break;
        }
    }

    // Extract the unit part
    while let Some(c) = size_chars.next() {
        unit_str.push(c);
    }

    // Parse the numeric part
    let size_num = f64::from_str(&size_num_str).map_err(|e| {
        error!("Failed to parse size number: {}", e);
        Status::internal(format!("Failed to parse size number: {}", e))
    })?;

    // Convert to bytes based on the unit
    let bytes = match unit_str.to_uppercase().as_str() {
        "B" => size_num,
        "K" => size_num * 1024.0,
        "M" => size_num * 1024.0 * 1024.0,
        "G" => size_num * 1024.0 * 1024.0 * 1024.0,
        "T" => size_num * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "P" => size_num * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => {
            error!("Unknown size unit: {}", unit_str);
            size_num
        }
    };

    Ok(bytes as u64)
}

/// Get all paths to a disk
fn get_disk_paths(device: &str) -> Result<Vec<String>, Status> {
    debug!("Getting paths for disk {}", device);
    let output = Command::new("/usr/bin/ls")
        .args(["-l", &format!("/dev/dsk/{}", device), "/dev/rdsk/{}", device])
        .output()
        .map_err(|e| {
            error!("Failed to execute ls command: {}", e);
            Status::internal(format!("Failed to execute ls command: {}", e))
        })?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        error!("ls command failed: {}", error_msg);
        return Err(Status::internal(format!("ls command failed: {}", error_msg)));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let paths = output_str
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    Ok(paths)
}

/// Execute the dladm command and parse its output
pub fn get_network_info() -> Result<Vec<NetworkInterface>, Status> {
    debug!("Executing dladm show-phys command");
    let output = Command::new("/usr/sbin/dladm")
        .args(["show-phys", "-m", "-o", "link,class,media,state,speed,over,mtu"])
        .output()
        .map_err(|e| {
            error!("Failed to execute dladm command: {}", e);
            Status::internal(format!("Failed to execute dladm command: {}", e))
        })?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        error!("dladm command failed: {}", error_msg);
        return Err(Status::internal(format!("dladm command failed: {}", error_msg)));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    parse_dladm_output(&output_str)
}

/// Parse the output of the dladm command
fn parse_dladm_output(output: &str) -> Result<Vec<NetworkInterface>, Status> {
    let mut interfaces = Vec::new();
    let mut lines = output.lines();

    // Skip the header line
    if let Some(_header) = lines.next() {
        for line in lines {
            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 7 {
                error!("Invalid dladm output line: {}", line);
                continue;
            }

            let link = fields[0].to_string();
            let class = fields[1].to_string();
            let media = fields[2].to_string();
            let state = fields[3].to_string();
            let speed = fields[4].to_string();
            let over_str = fields[5].to_string();
            let mtu = fields[6].to_string();

            // Parse over flag
            let over = !over_str.eq_ignore_ascii_case("--");

            // Get MAC address
            let mac_address = get_mac_address(&link).unwrap_or_else(|_| "".to_string());

            let interface = NetworkInterface {
                name: link.clone(),
                link,
                class,
                media,
                state,
                speed,
                mac_address,
                over,
                mtu,
            };

            interfaces.push(interface);
        }
    }

    Ok(interfaces)
}

/// Get the MAC address for a network interface
fn get_mac_address(link: &str) -> Result<String, Status> {
    debug!("Getting MAC address for interface {}", link);
    let output = Command::new("/usr/sbin/dladm")
        .args(["show-phys", "-m", link])
        .output()
        .map_err(|e| {
            error!("Failed to execute dladm command: {}", e);
            Status::internal(format!("Failed to execute dladm command: {}", e))
        })?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        error!("dladm command failed: {}", error_msg);
        return Err(Status::internal(format!("dladm command failed: {}", error_msg)));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.lines().collect();
    if lines.len() < 2 {
        return Ok("".to_string());
    }

    // The MAC address is in the second line, second column
    let fields: Vec<&str> = lines[1].split_whitespace().collect();
    if fields.len() < 2 {
        return Ok("".to_string());
    }

    Ok(fields[1].to_string())
}

/// Execute the smbios command and parse its output
pub fn get_smbios_info() -> Result<SmbiosInfo, Status> {
    debug!("Executing smbios command");
    let output = Command::new("/usr/sbin/smbios")
        .output()
        .map_err(|e| {
            error!("Failed to execute smbios command: {}", e);
            Status::internal(format!("Failed to execute smbios command: {}", e))
        })?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        error!("smbios command failed: {}", error_msg);
        return Err(Status::internal(format!("smbios command failed: {}", error_msg)));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    parse_smbios_output(&output_str)
}

/// Parse the output of the smbios command
fn parse_smbios_output(output: &str) -> Result<SmbiosInfo, Status> {
    // Create empty SMBIOS info structures
    let mut bios_info = BiosInfo::default();
    let mut system_info = SmbiosSystemInfo::default();
    let mut baseboard_info = BaseboardInfo::default();
    let mut chassis_info = ChassisInfo::default();
    let mut processors = Vec::new();
    let mut memory_arrays = Vec::new();
    let mut memory_devices = Vec::new();
    let mut memory_array_mapped_addresses = Vec::new();
    let mut boot_info = SystemBootInfo::default();

    // Split the output into sections based on the "ID    SIZE TYPE" header
    let sections: Vec<&str> = output.split("ID    SIZE TYPE").collect();
    
    // Skip the first section (it's empty)
    for section in sections.iter().skip(1) {
        // Parse the section based on the type
        if section.contains("SMB_TYPE_BIOS (type 0)") {
            bios_info = parse_bios_info(section)?;
        } else if section.contains("SMB_TYPE_SYSTEM (type 1)") {
            system_info = parse_system_info(section)?;
        } else if section.contains("SMB_TYPE_BASEBOARD (type 2)") {
            baseboard_info = parse_baseboard_info(section)?;
        } else if section.contains("SMB_TYPE_CHASSIS (type 3)") {
            chassis_info = parse_chassis_info(section)?;
        } else if section.contains("SMB_TYPE_PROCESSOR (type 4)") {
            let processor = parse_processor_info(section)?;
            processors.push(processor);
        } else if section.contains("SMB_TYPE_MEMARRAY (type 16)") {
            let memory_array = parse_memory_array_info(section)?;
            memory_arrays.push(memory_array);
        } else if section.contains("SMB_TYPE_MEMDEVICE (type 17)") {
            let memory_device = parse_memory_device_info(section)?;
            memory_devices.push(memory_device);
        } else if section.contains("SMB_TYPE_MEMARRAYMAP (type 19)") {
            let memory_array_mapped_address = parse_memory_array_mapped_address_info(section)?;
            memory_array_mapped_addresses.push(memory_array_mapped_address);
        } else if section.contains("SMB_TYPE_BOOT (type 32)") {
            boot_info = parse_boot_info(section)?;
        }
    }

    // Return the SMBIOS information
    Ok(SmbiosInfo {
        bios: Some(bios_info),
        system: Some(system_info),
        baseboard: Some(baseboard_info),
        chassis: Some(chassis_info),
        processors,
        memory_arrays,
        memory_devices,
        memory_array_mapped_addresses,
        boot: Some(boot_info),
    })
}

/// Parse BIOS information (type 0)
fn parse_bios_info(section: &str) -> Result<BiosInfo, Status> {
    let mut vendor = String::new();
    let mut version = String::new();
    let mut release_date = String::new();
    let mut address_segment = String::new();
    let mut rom_size = 0;
    let mut image_size = 0;
    let mut characteristics = 0;
    let mut characteristics_ext1 = 0;
    let mut characteristics_ext2 = 0;
    let mut version_number = String::new();

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Vendor:") {
            vendor = line.trim_start_matches("Vendor:").trim().to_string();
        } else if line.starts_with("Version String:") {
            version = line.trim_start_matches("Version String:").trim().to_string();
        } else if line.starts_with("Release Date:") {
            release_date = line.trim_start_matches("Release Date:").trim().to_string();
        } else if line.starts_with("Address Segment:") {
            address_segment = line.trim_start_matches("Address Segment:").trim().to_string();
        } else if line.starts_with("ROM Size:") {
            let rom_size_str = line.trim_start_matches("ROM Size:").trim();
            if let Some(size_str) = rom_size_str.split_whitespace().next() {
                rom_size = size_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Image Size:") {
            let image_size_str = line.trim_start_matches("Image Size:").trim();
            if let Some(size_str) = image_size_str.split_whitespace().next() {
                image_size = size_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Characteristics:") {
            let chars_str = line.trim_start_matches("Characteristics:").trim();
            if let Some(hex_str) = chars_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        characteristics = value;
                    }
                }
            }
        } else if line.starts_with("Characteristics Extension Byte 1:") {
            let chars_str = line.trim_start_matches("Characteristics Extension Byte 1:").trim();
            if let Some(hex_str) = chars_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        characteristics_ext1 = value;
                    }
                }
            }
        } else if line.starts_with("Characteristics Extension Byte 2:") {
            let chars_str = line.trim_start_matches("Characteristics Extension Byte 2:").trim();
            if let Some(hex_str) = chars_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        characteristics_ext2 = value;
                    }
                }
            }
        } else if line.starts_with("Version Number:") {
            version_number = line.trim_start_matches("Version Number:").trim().to_string();
        }
    }

    Ok(BiosInfo {
        vendor,
        version,
        release_date,
        address_segment,
        rom_size,
        image_size,
        characteristics,
        characteristics_ext1,
        characteristics_ext2,
        version_number,
    })
}

/// Parse System information (type 1)
fn parse_system_info(section: &str) -> Result<SmbiosSystemInfo, Status> {
    let mut manufacturer = String::new();
    let mut product = String::new();
    let mut version = String::new();
    let mut serial_number = String::new();
    let mut uuid = String::new();
    let mut wakeup_event = 0;
    let mut sku_number = String::new();
    let mut family = String::new();

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Manufacturer:") {
            manufacturer = line.trim_start_matches("Manufacturer:").trim().to_string();
        } else if line.starts_with("Product:") {
            product = line.trim_start_matches("Product:").trim().to_string();
        } else if line.starts_with("Version:") {
            version = line.trim_start_matches("Version:").trim().to_string();
        } else if line.starts_with("Serial Number:") {
            serial_number = line.trim_start_matches("Serial Number:").trim().to_string();
        } else if line.starts_with("UUID:") {
            uuid = line.trim_start_matches("UUID:").trim().to_string();
        } else if line.starts_with("Wake-Up Event:") {
            let event_str = line.trim_start_matches("Wake-Up Event:").trim();
            if let Some(hex_str) = event_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        wakeup_event = value;
                    }
                }
            }
        } else if line.starts_with("SKU Number:") {
            sku_number = line.trim_start_matches("SKU Number:").trim().to_string();
        } else if line.starts_with("Family:") {
            family = line.trim_start_matches("Family:").trim().to_string();
        }
    }

    Ok(SmbiosSystemInfo {
        manufacturer,
        product,
        version,
        serial_number,
        uuid,
        wakeup_event,
        sku_number,
        family,
    })
}

/// Parse Baseboard information (type 2)
fn parse_baseboard_info(section: &str) -> Result<BaseboardInfo, Status> {
    let mut manufacturer = String::new();
    let mut product = String::new();
    let mut version = String::new();
    let mut serial_number = String::new();
    let mut asset_tag = String::new();
    let mut location_tag = String::new();
    let mut chassis = 0;
    let mut flags = 0;
    let mut board_type = 0;

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Manufacturer:") {
            manufacturer = line.trim_start_matches("Manufacturer:").trim().to_string();
        } else if line.starts_with("Product:") {
            product = line.trim_start_matches("Product:").trim().to_string();
        } else if line.starts_with("Version:") {
            version = line.trim_start_matches("Version:").trim().to_string();
        } else if line.starts_with("Serial Number:") {
            serial_number = line.trim_start_matches("Serial Number:").trim().to_string();
        } else if line.starts_with("Asset Tag:") {
            asset_tag = line.trim_start_matches("Asset Tag:").trim().to_string();
        } else if line.starts_with("Location Tag:") {
            location_tag = line.trim_start_matches("Location Tag:").trim().to_string();
        } else if line.starts_with("Chassis:") {
            let chassis_str = line.trim_start_matches("Chassis:").trim();
            chassis = chassis_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Flags:") {
            let flags_str = line.trim_start_matches("Flags:").trim();
            if let Some(hex_str) = flags_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        flags = value;
                    }
                }
            }
        } else if line.starts_with("Board Type:") {
            let board_type_str = line.trim_start_matches("Board Type:").trim();
            if let Some(hex_str) = board_type_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        board_type = value;
                    }
                } else {
                    // Try to parse as decimal
                    if let Ok(value) = board_type_str.split_whitespace().next().unwrap_or("0").parse::<u32>() {
                        board_type = value;
                    }
                }
            }
        }
    }

    Ok(BaseboardInfo {
        manufacturer,
        product,
        version,
        serial_number,
        asset_tag,
        location_tag,
        chassis,
        flags,
        board_type,
    })
}

/// Parse Chassis information (type 3)
fn parse_chassis_info(section: &str) -> Result<ChassisInfo, Status> {
    let mut manufacturer = String::new();
    let mut version = String::new();
    let mut serial_number = String::new();
    let mut asset_tag = String::new();
    let mut oem_data = 0;
    let mut sku_number = String::new();
    let mut lock_present = false;
    let mut chassis_type = 0;
    let mut boot_up_state = 0;
    let mut power_supply_state = 0;
    let mut thermal_state = 0;
    let mut chassis_height = 0;
    let mut power_cords = 0;
    let mut element_records = 0;

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Manufacturer:") {
            manufacturer = line.trim_start_matches("Manufacturer:").trim().to_string();
        } else if line.starts_with("Version:") {
            version = line.trim_start_matches("Version:").trim().to_string();
        } else if line.starts_with("Serial Number:") {
            serial_number = line.trim_start_matches("Serial Number:").trim().to_string();
        } else if line.starts_with("Asset Tag:") {
            asset_tag = line.trim_start_matches("Asset Tag:").trim().to_string();
        } else if line.starts_with("OEM Data:") {
            let oem_data_str = line.trim_start_matches("OEM Data:").trim();
            if let Some(hex_str) = oem_data_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        oem_data = value;
                    }
                }
            }
        } else if line.starts_with("SKU Number:") {
            sku_number = line.trim_start_matches("SKU Number:").trim().to_string();
        } else if line.starts_with("Lock Present:") {
            let lock_present_str = line.trim_start_matches("Lock Present:").trim();
            lock_present = lock_present_str.eq_ignore_ascii_case("Y");
        } else if line.starts_with("Chassis Type:") {
            let chassis_type_str = line.trim_start_matches("Chassis Type:").trim();
            if let Some(hex_str) = chassis_type_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        chassis_type = value;
                    }
                } else {
                    // Try to parse as decimal
                    if let Ok(value) = chassis_type_str.split_whitespace().next().unwrap_or("0").parse::<u32>() {
                        chassis_type = value;
                    }
                }
            }
        } else if line.starts_with("Boot-Up State:") {
            let boot_up_state_str = line.trim_start_matches("Boot-Up State:").trim();
            if let Some(hex_str) = boot_up_state_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        boot_up_state = value;
                    }
                } else {
                    // Try to parse as decimal
                    if let Ok(value) = boot_up_state_str.split_whitespace().next().unwrap_or("0").parse::<u32>() {
                        boot_up_state = value;
                    }
                }
            }
        } else if line.starts_with("Power Supply State:") {
            let power_supply_state_str = line.trim_start_matches("Power Supply State:").trim();
            if let Some(hex_str) = power_supply_state_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        power_supply_state = value;
                    }
                } else {
                    // Try to parse as decimal
                    if let Ok(value) = power_supply_state_str.split_whitespace().next().unwrap_or("0").parse::<u32>() {
                        power_supply_state = value;
                    }
                }
            }
        } else if line.starts_with("Thermal State:") {
            let thermal_state_str = line.trim_start_matches("Thermal State:").trim();
            if let Some(hex_str) = thermal_state_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        thermal_state = value;
                    }
                } else {
                    // Try to parse as decimal
                    if let Ok(value) = thermal_state_str.split_whitespace().next().unwrap_or("0").parse::<u32>() {
                        thermal_state = value;
                    }
                }
            }
        } else if line.starts_with("Chassis Height:") {
            let chassis_height_str = line.trim_start_matches("Chassis Height:").trim();
            if let Some(height_str) = chassis_height_str.split_whitespace().next() {
                chassis_height = height_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Power Cords:") {
            let power_cords_str = line.trim_start_matches("Power Cords:").trim();
            power_cords = power_cords_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Element Records:") {
            let element_records_str = line.trim_start_matches("Element Records:").trim();
            element_records = element_records_str.parse::<u32>().unwrap_or(0);
        }
    }

    Ok(ChassisInfo {
        manufacturer,
        version,
        serial_number,
        asset_tag,
        oem_data,
        sku_number,
        lock_present,
        chassis_type,
        boot_up_state,
        power_supply_state,
        thermal_state,
        chassis_height,
        power_cords,
        element_records,
    })
}

/// Parse Processor information (type 4)
fn parse_processor_info(section: &str) -> Result<ProcessorInfo, Status> {
    let mut manufacturer = String::new();
    let mut version = String::new();
    let mut serial_number = String::new();
    let mut asset_tag = String::new();
    let mut location_tag = String::new();
    let mut part_number = String::new();
    let mut family = 0;
    let mut cpuid = 0;
    let mut processor_type = 0;
    let mut socket_upgrade = 0;
    let mut socket_populated = false;
    let mut processor_status = 0;
    let mut supported_voltages = String::new();
    let mut core_count = 0;
    let mut cores_enabled = 0;
    let mut thread_count = 0;
    let mut processor_characteristics = 0;
    let mut external_clock = String::new();
    let mut maximum_speed = String::new();
    let mut current_speed = String::new();
    let mut l1_cache_handle = 0;
    let mut l2_cache_handle = 0;
    let mut l3_cache_handle = 0;
    let mut threads_enabled = 0;

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Manufacturer:") {
            manufacturer = line.trim_start_matches("Manufacturer:").trim().to_string();
        } else if line.starts_with("Version:") {
            version = line.trim_start_matches("Version:").trim().to_string();
        } else if line.starts_with("Serial Number:") {
            serial_number = line.trim_start_matches("Serial Number:").trim().to_string();
        } else if line.starts_with("Asset Tag:") {
            asset_tag = line.trim_start_matches("Asset Tag:").trim().to_string();
        } else if line.starts_with("Location Tag:") {
            location_tag = line.trim_start_matches("Location Tag:").trim().to_string();
        } else if line.starts_with("Part Number:") {
            part_number = line.trim_start_matches("Part Number:").trim().to_string();
        } else if line.starts_with("Family:") {
            let family_str = line.trim_start_matches("Family:").trim();
            if let Some(value_str) = family_str.split_whitespace().next() {
                family = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("CPUID:") {
            let cpuid_str = line.trim_start_matches("CPUID:").trim();
            if let Some(hex_str) = cpuid_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        cpuid = value;
                    }
                }
            }
        } else if line.starts_with("Type:") {
            let type_str = line.trim_start_matches("Type:").trim();
            if let Some(value_str) = type_str.split_whitespace().next() {
                processor_type = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Socket Upgrade:") {
            let socket_upgrade_str = line.trim_start_matches("Socket Upgrade:").trim();
            if let Some(value_str) = socket_upgrade_str.split_whitespace().next() {
                socket_upgrade = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Socket Status:") {
            let socket_status_str = line.trim_start_matches("Socket Status:").trim();
            socket_populated = socket_status_str.contains("Populated");
        } else if line.starts_with("Processor Status:") {
            let processor_status_str = line.trim_start_matches("Processor Status:").trim();
            if let Some(value_str) = processor_status_str.split_whitespace().next() {
                processor_status = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Supported Voltages:") {
            supported_voltages = line.trim_start_matches("Supported Voltages:").trim().to_string();
        } else if line.starts_with("Core Count:") {
            let core_count_str = line.trim_start_matches("Core Count:").trim();
            core_count = core_count_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Cores Enabled:") {
            let cores_enabled_str = line.trim_start_matches("Cores Enabled:").trim();
            if cores_enabled_str != "Unknown" {
                cores_enabled = cores_enabled_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Thread Count:") {
            let thread_count_str = line.trim_start_matches("Thread Count:").trim();
            thread_count = thread_count_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Processor Characteristics:") {
            let chars_str = line.trim_start_matches("Processor Characteristics:").trim();
            if let Some(hex_str) = chars_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        processor_characteristics = value;
                    }
                }
            }
        } else if line.starts_with("External Clock Speed:") {
            external_clock = line.trim_start_matches("External Clock Speed:").trim().to_string();
        } else if line.starts_with("Maximum Speed:") {
            maximum_speed = line.trim_start_matches("Maximum Speed:").trim().to_string();
        } else if line.starts_with("Current Speed:") {
            current_speed = line.trim_start_matches("Current Speed:").trim().to_string();
        } else if line.starts_with("L1 Cache Handle:") {
            let l1_cache_handle_str = line.trim_start_matches("L1 Cache Handle:").trim();
            if l1_cache_handle_str != "None" {
                l1_cache_handle = l1_cache_handle_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("L2 Cache Handle:") {
            let l2_cache_handle_str = line.trim_start_matches("L2 Cache Handle:").trim();
            if l2_cache_handle_str != "None" {
                l2_cache_handle = l2_cache_handle_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("L3 Cache Handle:") {
            let l3_cache_handle_str = line.trim_start_matches("L3 Cache Handle:").trim();
            if l3_cache_handle_str != "None" {
                l3_cache_handle = l3_cache_handle_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Threads Enabled:") {
            let threads_enabled_str = line.trim_start_matches("Threads Enabled:").trim();
            if threads_enabled_str != "Unknown" {
                threads_enabled = threads_enabled_str.parse::<u32>().unwrap_or(0);
            }
        }
    }

    Ok(ProcessorInfo {
        manufacturer,
        version,
        serial_number,
        asset_tag,
        location_tag,
        part_number,
        family,
        cpuid,
        r#type: processor_type, // Use r#type because 'type' is a reserved keyword
        socket_upgrade,
        socket_populated,
        processor_status,
        supported_voltages,
        core_count,
        cores_enabled,
        thread_count,
        processor_characteristics,
        external_clock,
        maximum_speed,
        current_speed,
        l1_cache_handle,
        l2_cache_handle,
        l3_cache_handle,
        threads_enabled,
    })
}

/// Parse Memory Array information (type 16)
fn parse_memory_array_info(section: &str) -> Result<MemoryArrayInfo, Status> {
    let mut location = 0;
    let mut use_type = 0;
    let mut ecc = 0;
    let mut slots = 0;
    let mut max_capacity = 0;

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Location:") {
            let location_str = line.trim_start_matches("Location:").trim();
            if let Some(value_str) = location_str.split_whitespace().next() {
                location = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Use:") {
            let use_str = line.trim_start_matches("Use:").trim();
            if let Some(value_str) = use_str.split_whitespace().next() {
                use_type = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("ECC:") {
            let ecc_str = line.trim_start_matches("ECC:").trim();
            if let Some(value_str) = ecc_str.split_whitespace().next() {
                ecc = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Number of Slots/Sockets:") {
            let slots_str = line.trim_start_matches("Number of Slots/Sockets:").trim();
            slots = slots_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Max Capacity:") {
            let max_capacity_str = line.trim_start_matches("Max Capacity:").trim();
            if let Some(size_str) = max_capacity_str.split_whitespace().next() {
                max_capacity = size_str.parse::<u64>().unwrap_or(0);
            }
        }
    }

    Ok(MemoryArrayInfo {
        location,
        r#use: use_type, // Use r#use because 'use' is a reserved keyword
        ecc,
        slots,
        max_capacity,
    })
}

/// Parse Memory Device information (type 17)
fn parse_memory_device_info(section: &str) -> Result<MemoryDeviceInfo, Status> {
    let mut manufacturer = String::new();
    let mut serial_number = String::new();
    let mut asset_tag = String::new();
    let mut location_tag = String::new();
    let mut part_number = String::new();
    let mut array_handle = 0;
    let mut error_handle = 0;
    let mut total_width = 0;
    let mut data_width = 0;
    let mut size = 0;
    let mut form_factor = 0;
    let mut set = 0;
    let mut rank = 0;
    let mut memory_type = 0;
    let mut flags = 0;
    let mut speed = String::new();
    let mut configured_speed = String::new();
    let mut device_locator = String::new();
    let mut bank_locator = String::new();
    let mut min_voltage = String::new();
    let mut max_voltage = String::new();
    let mut configured_voltage = String::new();

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Manufacturer:") {
            manufacturer = line.trim_start_matches("Manufacturer:").trim().to_string();
        } else if line.starts_with("Serial Number:") {
            serial_number = line.trim_start_matches("Serial Number:").trim().to_string();
        } else if line.starts_with("Asset Tag:") {
            asset_tag = line.trim_start_matches("Asset Tag:").trim().to_string();
        } else if line.starts_with("Location Tag:") {
            location_tag = line.trim_start_matches("Location Tag:").trim().to_string();
        } else if line.starts_with("Part Number:") {
            part_number = line.trim_start_matches("Part Number:").trim().to_string();
        } else if line.starts_with("Physical Memory Array:") {
            let array_handle_str = line.trim_start_matches("Physical Memory Array:").trim();
            array_handle = array_handle_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Memory Error Data:") {
            let error_handle_str = line.trim_start_matches("Memory Error Data:").trim();
            if error_handle_str != "None" {
                error_handle = error_handle_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Total Width:") {
            let total_width_str = line.trim_start_matches("Total Width:").trim();
            if let Some(width_str) = total_width_str.split_whitespace().next() {
                total_width = width_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Data Width:") {
            let data_width_str = line.trim_start_matches("Data Width:").trim();
            if let Some(width_str) = data_width_str.split_whitespace().next() {
                data_width = width_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Size:") {
            let size_str = line.trim_start_matches("Size:").trim();
            if let Some(bytes_str) = size_str.split_whitespace().next() {
                size = bytes_str.parse::<u64>().unwrap_or(0);
            }
        } else if line.starts_with("Form Factor:") {
            let form_factor_str = line.trim_start_matches("Form Factor:").trim();
            if let Some(value_str) = form_factor_str.split_whitespace().next() {
                form_factor = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Set:") {
            let set_str = line.trim_start_matches("Set:").trim();
            if set_str != "None" {
                set = set_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Rank:") {
            let rank_str = line.trim_start_matches("Rank:").trim();
            if rank_str != "Unknown" {
                rank = rank_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Memory Type:") {
            let memory_type_str = line.trim_start_matches("Memory Type:").trim();
            if let Some(value_str) = memory_type_str.split_whitespace().next() {
                memory_type = value_str.parse::<u32>().unwrap_or(0);
            }
        } else if line.starts_with("Flags:") {
            let flags_str = line.trim_start_matches("Flags:").trim();
            if let Some(hex_str) = flags_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        flags = value;
                    }
                }
            }
        } else if line.starts_with("Speed:") {
            speed = line.trim_start_matches("Speed:").trim().to_string();
        } else if line.starts_with("Configured Speed:") {
            configured_speed = line.trim_start_matches("Configured Speed:").trim().to_string();
        } else if line.starts_with("Device Locator:") {
            device_locator = line.trim_start_matches("Device Locator:").trim().to_string();
        } else if line.starts_with("Bank Locator:") {
            bank_locator = line.trim_start_matches("Bank Locator:").trim().to_string();
        } else if line.starts_with("Minimum Voltage:") {
            min_voltage = line.trim_start_matches("Minimum Voltage:").trim().to_string();
        } else if line.starts_with("Maximum Voltage:") {
            max_voltage = line.trim_start_matches("Maximum Voltage:").trim().to_string();
        } else if line.starts_with("Configured Voltage:") {
            configured_voltage = line.trim_start_matches("Configured Voltage:").trim().to_string();
        }
    }

    Ok(MemoryDeviceInfo {
        manufacturer,
        serial_number,
        asset_tag,
        location_tag,
        part_number,
        array_handle,
        error_handle,
        total_width,
        data_width,
        size,
        form_factor,
        set,
        rank,
        memory_type,
        flags,
        speed,
        configured_speed,
        device_locator,
        bank_locator,
        min_voltage,
        max_voltage,
        configured_voltage,
    })
}

/// Parse Memory Array Mapped Address information (type 19)
fn parse_memory_array_mapped_address_info(section: &str) -> Result<MemoryArrayMappedAddressInfo, Status> {
    let mut array_handle = 0;
    let mut devices_per_row = 0;
    let mut physical_address = 0;
    let mut size = 0;

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Physical Memory Array:") {
            let array_handle_str = line.trim_start_matches("Physical Memory Array:").trim();
            array_handle = array_handle_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Devices per Row:") {
            let devices_per_row_str = line.trim_start_matches("Devices per Row:").trim();
            devices_per_row = devices_per_row_str.parse::<u32>().unwrap_or(0);
        } else if line.starts_with("Physical Address:") {
            let physical_address_str = line.trim_start_matches("Physical Address:").trim();
            if let Some(hex_str) = physical_address_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u64::from_str_radix(&hex_str[2..], 16) {
                        physical_address = value;
                    }
                } else {
                    // Try to parse as decimal
                    physical_address = physical_address_str.parse::<u64>().unwrap_or(0);
                }
            }
        } else if line.starts_with("Size:") {
            let size_str = line.trim_start_matches("Size:").trim();
            if let Some(bytes_str) = size_str.split_whitespace().next() {
                size = bytes_str.parse::<u64>().unwrap_or(0);
            }
        }
    }

    Ok(MemoryArrayMappedAddressInfo {
        array_handle,
        devices_per_row,
        physical_address,
        size,
    })
}

/// Parse System Boot information (type 32)
fn parse_boot_info(section: &str) -> Result<SystemBootInfo, Status> {
    let mut status_code = 0;

    // Parse each line in the section
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("Boot Status Code:") {
            let status_code_str = line.trim_start_matches("Boot Status Code:").trim();
            if let Some(hex_str) = status_code_str.split_whitespace().next() {
                if hex_str.starts_with("0x") {
                    if let Ok(value) = u32::from_str_radix(&hex_str[2..], 16) {
                        status_code = value;
                    }
                } else {
                    // Try to parse as decimal
                    status_code = status_code_str.parse::<u32>().unwrap_or(0);
                }
            }
        }
    }

    Ok(SystemBootInfo {
        status_code,
    })
}

/// Get system information
pub fn get_system_info() -> Result<SystemInfoResponse, Status> {
    let disks = get_disk_info()?;
    let network_interfaces = get_network_info()?;
    let smbios = get_smbios_info()?;
    let partitions = get_partitions_info(&disks).unwrap_or_else(|_| Vec::new());

    Ok(SystemInfoResponse {
        disks,
        network_interfaces,
        smbios: Some(smbios),
        partitions,
    })
}

/// Enumerate partitions/slices under /dev/dsk for each disk
fn get_partitions_info(disks: &Vec<DiskInfo>) -> Result<Vec<PartitionInfo>, Status> {
    let mut parts: Vec<PartitionInfo> = Vec::new();
    let entries = fs::read_dir("/dev/dsk").map_err(|e| Status::internal(format!("failed to read /dev/dsk: {}", e)))?;
    let mut names: Vec<String> = Vec::new();
    for ent in entries {
        if let Ok(de) = ent {
            if let Ok(name_os) = de.file_name().into_string() {
                names.push(name_os);
            }
        }
    }
    for d in disks.iter() {
        let prefix_s = format!("{}s", d.device);
        let prefix_p = format!("{}p", d.device);
        for n in names.iter() {
            let is_slice = n.starts_with(&prefix_s) && n[prefix_s.len()..].chars().all(|c| c.is_ascii_digit());
            let is_part = n.starts_with(&prefix_p) && n[prefix_p.len()..].chars().all(|c| c.is_ascii_digit());
            if is_slice || is_part {
                parts.push(PartitionInfo {
                    device: n.clone(),
                    size_bytes: 0,
                    parent_device: d.device.clone(),
                });
            }
        }
    }
    Ok(parts)
}