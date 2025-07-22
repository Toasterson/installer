# System Information Retrieval

The Illumos Installer provides a way to retrieve system information from a remote machined daemon via gRPC. This functionality is useful for gathering information about the system before installation, such as available disks and network interfaces.

## Using the SystemInfo Command

The `installadm` tool includes a `system-info` command that retrieves system information from a machined server:

```bash
installadm system-info <server-name>
```

Where `<server-name>` is the name of a previously claimed server.

### Example

```bash
# First, claim a server
installadm claim http://192.168.1.100:50051 --secret password --name myserver

# Then, retrieve system information
installadm system-info myserver
```

This will display information about the disks, network interfaces, and SMBIOS data on the remote system:

```
Retrieving system information from server: myserver

Disk Information:
Device     Vendor          Product               Size            Removable   SSD        Serial           FLT   LOC   Location
--------------------------------------------------------------------------------------------------------------------
c0t0d0     ATA            SAMSUNG SSD 850        500.0 GB        No          Yes        S2RANX0J500001   -     -     -
c0t1d0     ATA            WDC WD10EZEX-00BN     1.0 TB          No          No         WD-WCC6Y5NSXK11   -     -     0,1

Network Interface Information:
Name       Class      Media      State      Speed           MTU        MAC Address         
----------------------------------------------------------------------------------------------------
net0       phys       Ethernet   up         1000Mb/s        1500       00:1b:21:c0:8f:5d
net1       phys       Ethernet   down       0Mb/s           1500       00:1b:21:c0:8f:5e

BIOS Information:
Vendor: BHYVE
Version: 14.0
Release Date: 10/10/2021
Address Segment: 0xf000
ROM Size: 65536 bytes
Image Size: 65536 bytes
Characteristics: 0x89090
Characteristics Extension Byte 1: 0x1
Characteristics Extension Byte 2: 0x11
Version Number: 0.0

System Information:
Manufacturer: OpenIndiana
Product: OpenIndiana HVM
Version: 1.0
Serial Number: a03828d6-aaf6-6dec-e075-d3bbc67a737f
UUID: d62838a0-f6aa-ec6d-e075-d3bbc67a737f
Wake-Up Event: 0x6
SKU Number: 001
Family: Virtual Machine

Baseboard Information:
Manufacturer: illumos
Product: BHYVE
Version: 1.0
Serial Number: None
Asset Tag: None
Location Tag: None
Chassis: 3
Flags: 0x1
Board Type: 0xa

Chassis Information:
Manufacturer: illumos
Version: 1.0
Serial Number: None
Asset Tag: None
OEM Data: 0x0
SKU Number: None
Lock Present: No
Chassis Type: 0x2
Boot-Up State: 0x3
Power Supply State: 0x3
Thermal State: 0x3
Chassis Height: 0u
Power Cords: 0
Element Records: 0

Processor Information:
Processor #0
  Manufacturer: 
  Version: 
  Serial Number: None
  Asset Tag: None
  Location Tag: CPU #0
  Part Number: None
  Family: 1
  CPUID: 0x0
  Type: 3
  Socket Upgrade: 6
  Socket Populated: Yes
  Processor Status: 1
  Supported Voltages: 
  Core Count: 1
  Cores Enabled: 0
  Thread Count: 1
  Processor Characteristics: 0x4
  External Clock Speed: Unknown
  Maximum Speed: Unknown
  Current Speed: Unknown
  L1 Cache Handle: 0
  L2 Cache Handle: 0
  L3 Cache Handle: 0
  Threads Enabled: 0

Memory Array Information:
Memory Array #0
  Location: 3
  Use: 3
  ECC: 3
  Number of Slots/Sockets: 2
  Max Capacity: 17179869184 bytes

Memory Device Information:
Memory Device #0
  Manufacturer: 
  Serial Number: None
  Asset Tag: None
  Location Tag: 
  Part Number: None
  Physical Memory Array: 12
  Memory Error Data: 0
  Total Width: 64 bits
  Data Width: 64 bits
  Size: 17179869184 bytes
  Form Factor: 2
  Set: 0
  Rank: 0
  Memory Type: 2
  Flags: 0x4
  Speed: Unknown
  Configured Speed: Unknown
  Device Locator: 
  Bank Locator: 
  Minimum Voltage: Unknown
  Maximum Voltage: Unknown
  Configured Voltage: Unknown

System Boot Information:
Boot Status Code: 0x0
```

The output includes:

Disk Information:
- `FLT`: Fault status of the disk
- `LOC`: Location code of the disk
- `Location`: Physical location (chassis and bay) of the disk, if known

SMBIOS Information:
- BIOS information (vendor, version, release date, etc.)
- System information (manufacturer, product, version, serial number, etc.)
- Baseboard information (manufacturer, product, version, etc.)
- Chassis information (manufacturer, version, type, etc.)
- Processor information (manufacturer, family, type, core count, etc.)
- Memory information (arrays, devices, mapped addresses)
- Boot information (status code)

## Programmatic Access

The system information can also be accessed programmatically via the gRPC API. The API provides a `GetSystemInfo` method that returns a `SystemInfoResponse` containing information about disks and network interfaces.

### Protocol Definition

```protobuf
syntax = "proto3";

package machined;

// Request message for GetSystemInfo RPC
message SystemInfoRequest {
  // Empty for now, may add filters or options in the future
}

// Disk information structure
message DiskInfo {
  string device = 1;        // Device name (e.g., c0t0d0)
  string vendor = 2;        // Vendor name
  string product = 3;       // Product name
  string serial = 4;        // Serial number
  uint64 size_bytes = 5;    // Size in bytes
  bool removable = 6;       // Whether the disk is removable
  bool solid_state = 7;     // Whether the disk is solid state (SSD)
  repeated string paths = 8; // All paths to this disk
  string fault_status = 9;  // Fault status (FLT column in diskinfo -P output)
  string location_code = 10; // Location code (LOC column in diskinfo -P output)
  string chassis_bay = 11;  // Physical location (LOCATION column in diskinfo -P output)
}

// Network interface information structure
message NetworkInterface {
  string name = 1;          // Interface name (e.g., net0)
  string link = 2;          // Physical link name
  string class = 3;         // Interface class
  string media = 4;         // Media type
  string state = 5;         // Link state
  string speed = 6;         // Link speed
  string mac_address = 7;   // MAC address
  bool over = 8;            // Whether this is a virtual interface
  string mtu = 9;           // MTU size
}

// BIOS information structure (SMBIOS Type 0)
message BiosInfo {
  string vendor = 1;           // Vendor name
  string version = 2;          // Version string
  string release_date = 3;     // Release date
  string address_segment = 4;  // Address segment
  uint32 rom_size = 5;         // ROM size in bytes
  uint32 image_size = 6;       // Image size in bytes
  uint32 characteristics = 7;  // BIOS characteristics
  uint32 characteristics_ext1 = 8; // Characteristics extension byte 1
  uint32 characteristics_ext2 = 9; // Characteristics extension byte 2
  string version_number = 10;  // Version number
}

// System information structure (SMBIOS Type 1)
message SystemInfo {
  string manufacturer = 1;     // Manufacturer name
  string product = 2;          // Product name
  string version = 3;          // Version
  string serial_number = 4;    // Serial number
  string uuid = 5;             // UUID
  uint32 wakeup_event = 6;     // Wake-up event
  string sku_number = 7;       // SKU number
  string family = 8;           // Family
}

// Baseboard information structure (SMBIOS Type 2)
message BaseboardInfo {
  string manufacturer = 1;     // Manufacturer name
  string product = 2;          // Product name
  string version = 3;          // Version
  string serial_number = 4;    // Serial number
  string asset_tag = 5;        // Asset tag
  string location_tag = 6;     // Location tag
  uint32 chassis = 7;          // Chassis
  uint32 flags = 8;            // Flags
  uint32 board_type = 9;       // Board type
}

// Chassis information structure (SMBIOS Type 3)
message ChassisInfo {
  string manufacturer = 1;     // Manufacturer name
  string version = 2;          // Version
  string serial_number = 3;    // Serial number
  string asset_tag = 4;        // Asset tag
  uint32 oem_data = 5;         // OEM data
  string sku_number = 6;       // SKU number
  bool lock_present = 7;       // Lock present
  uint32 chassis_type = 8;     // Chassis type
  uint32 boot_up_state = 9;    // Boot-up state
  uint32 power_supply_state = 10; // Power supply state
  uint32 thermal_state = 11;   // Thermal state
  uint32 chassis_height = 12;  // Chassis height
  uint32 power_cords = 13;     // Power cords
  uint32 element_records = 14; // Element records
}

// Processor information structure (SMBIOS Type 4)
message ProcessorInfo {
  string manufacturer = 1;     // Manufacturer name
  string version = 2;          // Version
  string serial_number = 3;    // Serial number
  string asset_tag = 4;        // Asset tag
  string location_tag = 5;     // Location tag
  string part_number = 6;      // Part number
  uint32 family = 7;           // Family
  uint32 cpuid = 8;            // CPUID
  uint32 type = 9;             // Type
  uint32 socket_upgrade = 10;  // Socket upgrade
  bool socket_populated = 11;  // Socket populated
  uint32 processor_status = 12; // Processor status
  string supported_voltages = 13; // Supported voltages
  uint32 core_count = 14;      // Core count
  uint32 cores_enabled = 15;   // Cores enabled
  uint32 thread_count = 16;    // Thread count
  uint32 processor_characteristics = 17; // Processor characteristics
  string external_clock = 18;  // External clock speed
  string maximum_speed = 19;   // Maximum speed
  string current_speed = 20;   // Current speed
  uint32 l1_cache_handle = 21; // L1 cache handle
  uint32 l2_cache_handle = 22; // L2 cache handle
  uint32 l3_cache_handle = 23; // L3 cache handle
  uint32 threads_enabled = 24; // Threads enabled
}

// Memory array information structure (SMBIOS Type 16)
message MemoryArrayInfo {
  uint32 location = 1;         // Location
  uint32 use = 2;              // Use
  uint32 ecc = 3;              // ECC
  uint32 slots = 4;            // Number of slots/sockets
  uint64 max_capacity = 5;     // Maximum capacity in bytes
}

// Memory device information structure (SMBIOS Type 17)
message MemoryDeviceInfo {
  string manufacturer = 1;     // Manufacturer name
  string serial_number = 2;    // Serial number
  string asset_tag = 3;        // Asset tag
  string location_tag = 4;     // Location tag
  string part_number = 5;      // Part number
  uint32 array_handle = 6;     // Physical memory array handle
  uint32 error_handle = 7;     // Memory error data handle
  uint32 total_width = 8;      // Total width in bits
  uint32 data_width = 9;       // Data width in bits
  uint64 size = 10;            // Size in bytes
  uint32 form_factor = 11;     // Form factor
  uint32 set = 12;             // Set
  uint32 rank = 13;            // Rank
  uint32 memory_type = 14;     // Memory type
  uint32 flags = 15;           // Flags
  string speed = 16;           // Speed
  string configured_speed = 17; // Configured speed
  string device_locator = 18;  // Device locator
  string bank_locator = 19;    // Bank locator
  string min_voltage = 20;     // Minimum voltage
  string max_voltage = 21;     // Maximum voltage
  string configured_voltage = 22; // Configured voltage
}

// Memory array mapped address information structure (SMBIOS Type 19)
message MemoryArrayMappedAddressInfo {
  uint32 array_handle = 1;     // Physical memory array handle
  uint32 devices_per_row = 2;  // Devices per row
  uint64 physical_address = 3; // Physical address
  uint64 size = 4;             // Size in bytes
}

// System boot information structure (SMBIOS Type 32)
message SystemBootInfo {
  uint32 status_code = 1;      // Boot status code
}

// SMBIOS information structure
message SmbiosInfo {
  BiosInfo bios = 1;                  // BIOS information
  SystemInfo system = 2;              // System information
  BaseboardInfo baseboard = 3;        // Baseboard information
  ChassisInfo chassis = 4;            // Chassis information
  repeated ProcessorInfo processors = 5; // Processor information
  repeated MemoryArrayInfo memory_arrays = 6; // Memory array information
  repeated MemoryDeviceInfo memory_devices = 7; // Memory device information
  repeated MemoryArrayMappedAddressInfo memory_array_mapped_addresses = 8; // Memory array mapped address information
  SystemBootInfo boot = 9;            // System boot information
}

// Response message for GetSystemInfo RPC
message SystemInfoResponse {
  repeated DiskInfo disks = 1;                // List of disks
  repeated NetworkInterface network_interfaces = 2; // List of network interfaces
  SmbiosInfo smbios = 3;                     // SMBIOS information
}

service MachineService {
  // ... other methods ...
  rpc GetSystemInfo(SystemInfoRequest) returns (SystemInfoResponse);
}
```

Note: This uses proto3 syntax, which doesn't require field labels (optional, required, or repeated) except for repeated fields.

### Example Client Code

```rust
use machined::machine_service_client::MachineServiceClient;
use machined::SystemInfoRequest;
use tonic::transport::Channel;

async fn get_system_info(client: &mut MachineServiceClient<Channel>) -> Result<(), Box<dyn std::error::Error>> {
    let request = tonic::Request::new(SystemInfoRequest {});
    let response = client.get_system_info(request).await?;
    let system_info = response.into_inner();
    
    println!("Disks:");
    for disk in system_info.disks {
        println!("  Device: {}", disk.device);
        println!("  Vendor: {}", disk.vendor);
        println!("  Product: {}", disk.product);
        println!("  Serial: {}", disk.serial);
        println!("  Size: {} bytes", disk.size_bytes);
        println!("  Removable: {}", disk.removable);
        println!("  SSD: {}", disk.solid_state);
        println!("  Fault Status: {}", disk.fault_status);
        println!("  Location Code: {}", disk.location_code);
        println!("  Chassis/Bay: {}", disk.chassis_bay);
        println!("  Paths:");
        for path in disk.paths {
            println!("    {}", path);
        }
        println!();
    }
    
    println!("Network Interfaces:");
    for interface in system_info.network_interfaces {
        println!("  Name: {}", interface.name);
        println!("  Link: {}", interface.link);
        println!("  Class: {}", interface.class);
        println!("  Media: {}", interface.media);
        println!("  State: {}", interface.state);
        println!("  Speed: {}", interface.speed);
        println!("  MAC Address: {}", interface.mac_address);
        println!("  Virtual: {}", interface.over);
        println!("  MTU: {}", interface.mtu);
        println!();
    }
    
    // Display SMBIOS information if available
    if let Some(smbios) = &system_info.smbios {
        // Display BIOS information
        if let Some(bios) = &smbios.bios {
            println!("BIOS Information:");
            println!("  Vendor: {}", bios.vendor);
            println!("  Version: {}", bios.version);
            println!("  Release Date: {}", bios.release_date);
            println!("  Address Segment: {}", bios.address_segment);
            println!("  ROM Size: {} bytes", bios.rom_size);
            println!("  Image Size: {} bytes", bios.image_size);
            println!("  Characteristics: 0x{:x}", bios.characteristics);
            println!();
        }
        
        // Display System information
        if let Some(system) = &smbios.system {
            println!("System Information:");
            println!("  Manufacturer: {}", system.manufacturer);
            println!("  Product: {}", system.product);
            println!("  Version: {}", system.version);
            println!("  Serial Number: {}", system.serial_number);
            println!("  UUID: {}", system.uuid);
            println!();
        }
        
        // Display Processor information
        if !smbios.processors.is_empty() {
            println!("Processors:");
            for (i, processor) in smbios.processors.iter().enumerate() {
                println!("  Processor #{}", i);
                println!("    Manufacturer: {}", processor.manufacturer);
                println!("    Version: {}", processor.version);
                println!("    Location Tag: {}", processor.location_tag);
                println!("    Family: {}", processor.family);
                println!("    Core Count: {}", processor.core_count);
                println!("    Thread Count: {}", processor.thread_count);
                println!();
            }
        }
        
        // Display Memory information
        if !smbios.memory_arrays.is_empty() {
            println!("Memory Arrays:");
            for (i, memory_array) in smbios.memory_arrays.iter().enumerate() {
                println!("  Memory Array #{}", i);
                println!("    Location: {}", memory_array.location);
                println!("    Use: {}", memory_array.r#use);
                println!("    Number of Slots: {}", memory_array.slots);
                println!("    Max Capacity: {} bytes", memory_array.max_capacity);
                println!();
            }
        }
    }
    
    Ok(())
}
```

## Testing the Implementation

To test the system information retrieval functionality:

1. Build the machined and installadm crates:

```bash
cd /home/toasty/ws/illumos/installer/machined
cargo build

cd /home/toasty/ws/illumos/installer/installadm
cargo build
```

2. Start the machined daemon:

```bash
cd /home/toasty/ws/illumos/installer/machined
cargo run
```

3. In another terminal, use installadm to claim the machined server:

```bash
cd /home/toasty/ws/illumos/installer/installadm
cargo run -- claim http://localhost:50051 --secret password --name myserver
```

4. Use installadm to retrieve system information from the server:

```bash
cd /home/toasty/ws/illumos/installer/installadm
cargo run -- system-info myserver
```

This should display information about the disks and network interfaces on the system.

## Troubleshooting

If you encounter issues with the system information retrieval:

1. Ensure that the machined daemon is running and accessible
2. Check that you have claimed the server successfully
3. Verify that the server name is correct
4. Check the machined logs for any error messages
5. Ensure that the diskinfo, dladm, and smbios commands are available on the system