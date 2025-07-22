# Glossary

This glossary provides definitions for terms used throughout the illumos Installer documentation.

## A

### Addrconf
A method of IPv6 address configuration that uses Stateless Address Autoconfiguration (SLAAC) to automatically configure IPv6 addresses based on the network prefix and the interface's MAC address.

### Address
A network address assigned to a network interface, such as an IPv4 or IPv6 address.

## B

### Boot Environment
A bootable instance of the illumos operating system, including the root filesystem and any mounted filesystems. Boot environments allow for safe system updates and rollbacks.

### Bridge
A network device that connects multiple network segments, allowing them to communicate as if they were a single network.

## D

### DHCP (Dynamic Host Configuration Protocol)
A network protocol used to automatically assign IP addresses and other network configuration parameters to devices on a network.

### Dhcp4
A method of IPv4 address configuration that uses DHCP to automatically obtain an IPv4 address and other network configuration parameters.

### Dhcp6
A method of IPv6 address configuration that uses DHCPv6 to automatically obtain an IPv6 address and other network configuration parameters.

### DNS (Domain Name System)
A hierarchical and decentralized naming system for computers, services, or other resources connected to the Internet or a private network. It translates domain names to IP addresses.

## F

### FQDN (Fully Qualified Domain Name)
A domain name that specifies the exact location of a host within the DNS hierarchy. It includes the hostname and all domain levels, e.g., `host.example.com`.

## G

### Gateway
A network device that serves as an entry point to another network. In the context of routing, it's the IP address of the router that connects a local network to other networks.

### gRPC
A high-performance, open-source universal RPC (Remote Procedure Call) framework developed by Google. It's used for communication between the SysConfig service and plugins.

## H

### Hostname
A label assigned to a device connected to a computer network. It serves as a human-readable identifier for the device.

## I

### Interface
A network interface that connects a computer to a network. It can be a physical device (like an Ethernet card) or a virtual device (like a VLAN or bridge interface).

### IP Address
A numerical label assigned to each device connected to a computer network that uses the Internet Protocol for communication. IPv4 addresses are 32-bit numbers, while IPv6 addresses are 128-bit numbers.

### IPv4
Internet Protocol version 4, the fourth version of the Internet Protocol, which uses 32-bit addresses in the format `xxx.xxx.xxx.xxx` (e.g., `192.168.1.1`).

### IPv6
Internet Protocol version 6, the most recent version of the Internet Protocol, which uses 128-bit addresses in the format `xxxx:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx:xxxx` (e.g., `2001:db8::1`).

## K

### KDL (Kubernetes Definition Language)
A document language with a syntax inspired by Rust, JavaScript, and TOML. It's used for Machine Configuration files.

### knus
A Rust library used for parsing KDL files in the Machine Configuration component.

## M

### MAC Address
Media Access Control address, a unique identifier assigned to a network interface controller (NIC) for use as a network address in communications within a network segment.

### Machine Configuration
A component of the illumos Installer that defines the overall configuration of a system, including storage, system image, and boot environment.

## N

### Nameserver
A server that translates domain names to IP addresses. Also known as a DNS server.

### Network Interface
See Interface.

## O

### OCI (Open Container Initiative)
An open governance structure for creating open industry standards around container formats and runtimes. OCI URLs are used to specify system images in Machine Configuration.

## P

### Plugin
A component that extends the functionality of the System Configuration service. Plugins are responsible for managing specific aspects of system configuration.

### Pool
A ZFS storage pool, which is a collection of virtual devices that provides physical storage space for filesystems.

## R

### RAID-Z
A data protection technology that combines aspects of RAID and ZFS. It provides similar protection to RAID-5 but without the "write hole" vulnerability.

## S

### Selector
A mechanism used to identify hardware components based on their attributes, such as MAC address or driver. Selectors are used in System Configuration to identify network interfaces.

### SLAAC (Stateless Address Autoconfiguration)
A method where a device can generate its own IPv6 address using a combination of locally available information and information advertised by routers.

### Static
A method of address configuration where the IP address and other network parameters are manually specified rather than automatically obtained.

### System Configuration
A component of the illumos Installer that manages system settings such as hostname, network configuration, and other aspects of system configuration through a plugin-based architecture.

### SysConfig
Short for System Configuration.

## U

### Unix Socket
A data communications endpoint for exchanging data between processes executing on the same host operating system. It's used for communication between the SysConfig service and plugins.

## V

### vdev (Virtual Device)
A virtual device in ZFS that represents physical storage devices or copies of data. vdevs can be individual disks, mirrors, RAID-Z groups, or other configurations.

### VLAN (Virtual Local Area Network)
A logical subnetwork that groups a collection of devices from different physical LANs. VLANs allow network administrators to logically segment a LAN without physically reconfiguring the network.

## Z

### ZFS
Z File System, a combined file system and logical volume manager designed by Sun Microsystems. It provides features such as protection against data corruption, support for high storage capacities, and efficient data compression.

### ZIL (ZFS Intent Log)
A component of ZFS that ensures data integrity in the event of a system crash or power failure. It records "intent" to change file system data structures before they are actually changed.