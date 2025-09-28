

# **A Unified Configuration Schema for Cloud Instance Initialization: Deconstructing Cloud-Init for a Modern, Type-Safe Abstraction**

## **Part I: Deconstruction of the Cloud-Init Configuration Model**

The design and implementation of a robust, declarative configuration system for cloud instance initialization requires a foundational deconstruction of the existing industry standard, cloud-init. While cloud-init is a powerful and ubiquitous tool, its configuration model is a product of organic evolution, resulting in a design with significant architectural deficiencies, including functional redundancy, inconsistent data structures, and a high cognitive load for developers.1 This section provides a rigorous analysis of the  
cloud-init ecosystem, dissecting its execution architecture, classifying its extensive module set, and identifying the specific design flaws that necessitate a new, unified abstraction. This analysis serves as the essential predicate for the prescriptive design proposed in the second part of this report.

### **Section 1.1: Core Concepts and Execution Architecture**

Understanding the temporal and hierarchical aspects of cloud-init's operation is a prerequisite for comprehending the purpose, scope, and limitations of its configuration modules. The system's behavior is governed by a multi-stage boot process, a set of module execution frequencies, and a clear hierarchy for merging configuration data. These elements, while effective, expose implementation details that complicate the user-facing configuration model.

#### **The Multi-Stage Boot Process**

The cloud-init service executes its tasks in a precise, multi-stage sequence during the system's boot process to ensure that dependencies are met—for example, ensuring network connectivity before attempting to download packages.3 This sequence is divided into five distinct operational stages:

1. **Generator Stage:** Early in the boot process, a systemd generator determines if cloud-init should run at all on the current boot.  
2. **Local Stage:** cloud-init locates local data sources (such as a "NoCloud" data source) without requiring network access.  
3. **Network Stage:** The system's network interfaces are configured and brought online based on the provided network configuration.  
4. **Config Stage:** The majority of system configuration modules are executed.  
5. **Final Stage:** Final tasks, such as running user scripts or installing packages, are performed.3

These five stages are mapped to three primary module execution *phases*, which are explicitly defined in the main configuration file, /etc/cloud/cloud.cfg. These phases dictate the order in which modules are run:

* cloud\_init\_modules: Corresponds to the network phase.  
* cloud\_config\_modules: Corresponds to the configuration phase.  
* cloud\_final\_modules: Corresponds to the final phase.3

The rigid, procedural nature of this architecture is a direct reflection of the necessities of system bootstrapping. However, this implementation detail has been allowed to leak into the configuration interface. A user must possess an implicit understanding of this sequence to correctly structure their configuration. For instance, a command placed in the bootcmd module (which runs very early) cannot rely on network resources that are only configured by modules in a later phase. This forces the user to adopt a procedural mindset ("first, this module must run to enable the network, then that module can run to download a file") rather than a declarative one ("the system state should include this network configuration and this file"). A modern configuration model should abstract this procedural complexity, allowing the user to declare a target state and empowering the underlying tooling to determine the correct operational sequence.

#### **Module Execution Frequencies**

Beyond the execution order, cloud-init provides granular control over the cadence at which individual modules run. This behavior is governed by a module's frequency property, which defines its idempotency and persistence across reboots and instance cloning.5 The three possible frequency values are:

* **PER\_INSTANCE:** The module runs on the first boot of a new instance. An instance is uniquely identified by its instance ID, which is provided by the cloud metadata service. If an instance is cloned or a new instance is created from a snapshot, it receives a new instance ID, and modules with this frequency will run again.4 This is the most common frequency for tasks like setting a hostname or creating initial user accounts.  
* **ONCE:** The module runs only a single time on a given image. Even if the instance is cloned, these modules will not run again on the clones. This frequency is suitable for tasks that should only ever happen once, such as generating unique SSH host keys for the base image itself.4  
* **PER\_ALWAYS:** The module runs on every single boot of the instance.4 This is appropriate for tasks that need to be enforced continuously, such as a script that checks system health or ensures a specific service is running.

This frequency model provides essential control over the lifecycle of configuration tasks. However, it is another layer of cloud-init's internal architecture that users are required to understand. A unified schema must provide a more intuitive and explicit way to declare the desired persistence of an operation without forcing the user to memorize these internal cloud-init terms.

#### **The Hierarchy of Configuration Data**

A single, pristine cloud image can be customized for a multitude of users and environments through a hierarchical data model. cloud-init consumes and merges configuration data from three primary sources, with later sources overriding earlier ones:

1. **Metadata:** This data is provided by the cloud platform's metadata service and includes fundamental information about the instance, such as its unique instance ID, hostname, and network configuration.1  
2. **Vendor Data:** This is an optional data source provided by the creator of the cloud image or the cloud provider. It can be used to apply baseline configurations or customizations specific to the provider's environment.1  
3. **User Data:** This is the primary mechanism for user-driven customization. At instance launch, a user provides a script or configuration file (most commonly in the \#cloud-config YAML format) that specifies the desired state of the machine.1

Directives specified in the user data will override any default configurations found in /etc/cloud/cloud.cfg and /etc/cloud/cloud.cfg.d/\*.cfg on the instance itself.3 The central goal of this report is to design a superior, unified language for this  
user-data field, thereby providing a more robust and developer-friendly interface for instance customization.

### **Section 1.2: A Functional Taxonomy of Cloud-Init Modules**

The configuration surface of cloud-init is vast, comprising over 60 distinct modules, each activated by one or more top-level keys in the cloud-config YAML file.7 This flat, non-hierarchical list of directives is a significant source of complexity, obscuring the logical, domain-oriented nature of instance initialization. It forces developers to learn a large, unstructured API surface rather than navigating a more intuitive, tree-like model.  
The first step toward creating a structured and unified configuration object is to classify this sprawling list of modules into logical domains. This act of creating a functional taxonomy reveals the underlying structure that is absent in the cloud-init YAML format itself. The modules can be grouped into the following categories, which form a natural blueprint for a more organized, compositional schema.

* **System Identity & Environment:** Modules that define the machine's core identity and operating environment.  
  * **Modules:** set\_hostname, update\_hostname, update\_etc\_hosts, timezone, locale, keyboard.  
  * **Function:** These modules are responsible for configuring the system's name, Fully Qualified Domain Name (FQDN), time zone, default language, and keyboard layout, establishing its basic identity on the network and for its users.3  
* **User and Access Control:** Modules focused on creating and managing user accounts and their primary means of remote access.  
  * **Modules:** users\_groups, ssh, ssh\_import\_id, ssh\_authkey\_fingerprints.  
  * **Function:** This group handles the creation of users and groups, configuration of the SSH daemon, and the management of SSH public keys. This includes importing keys from online services like Launchpad (lp:) and GitHub (gh:).6  
* **Authentication & Secrets Management:** Modules that handle user passwords and system-wide trust stores.  
  * **Modules:** set\_passwords (which handles both the password and chpasswd keys), ca\_certs.  
  * **Function:** These modules are responsible for setting user passwords, controlling password expiration policies, and managing the system's list of trusted Certificate Authorities (CAs).7 This category is the primary source of the functional redundancy that will be analyzed in detail in Section 1.3.  
* **Software Provisioning & Package Management:** Modules dedicated to installing, updating, and configuring system software.  
  * **Modules:** package\_update\_upgrade\_install, apt\_configure, apt\_pipelining, yum\_add\_repo, apk\_configure, snap, rh\_subscription.  
  * **Function:** This comprehensive suite of tools manages the lifecycle of software packages. It can update the package database, upgrade existing packages, install new ones from standard repositories, and configure package manager sources (e.g., PPAs, custom repositories, proxies, and GPG keys) for a wide variety of Linux distributions.7  
* **Storage & Filesystem Configuration:** Modules for preparing, partitioning, and mounting storage devices.  
  * **Modules:** disk\_setup, fs\_setup, growpart, resizefs, mounts.  
  * **Function:** These modules manage the instance's storage layout. They can create partition tables (gpt or mbr), format filesystems (ext4, xfs, etc.), automatically expand the root partition to utilize all available disk space, and configure persistent mount points in /etc/fstab.7  
* **Network Configuration:** Modules that configure network-related services. It is important to note that the primary configuration of network interfaces (IP addresses, gateways, etc.) is typically handled by a separate network-config data source, not the user-data cloud-config format. However, some cloud-config modules influence networking services.  
  * **Modules:** resolv\_conf, fan.  
  * **Function:** These modules manage DNS resolver settings in /etc/resolv.conf and configure specialized overlay networks like Ubuntu Fan.7  
* **Content & File Management:** Modules for writing arbitrary content to the filesystem.  
  * **Modules:** write\_files.  
  * **Function:** This highly flexible module allows for the creation of arbitrary files at any location on the disk. It supports various content encodings (e.g., b64, gzip) and allows for the specification of file permissions and ownership.7  
* **Imperative Execution Hooks:** Modules that provide an "escape hatch" for running arbitrary shell commands or scripts at different points in the boot process.  
  * **Modules:** bootcmd, runcmd, scripts\_per\_boot, scripts\_per\_instance, scripts\_per\_once, scripts\_user, scripts\_vendor.  
  * **Function:** These modules are essential for performing actions not covered by cloud-init's declarative modules. bootcmd executes very early in the boot process, before most other modules, while runcmd executes late in the final phase, after package installation and other configurations are complete.6  
* **Third-Party Integrations & Configuration Management:** Modules designed to bootstrap more advanced, full-featured configuration management (CM) tools.  
  * **Modules:** ansible, puppet, chef, salt\_minion, mcollective, landscape, lxd.  
  * **Function:** These modules typically install the agent for a given CM tool (e.g., ansible, puppet-agent) and perform initial configuration, effectively handing off the long-term management of the instance to a more powerful and specialized system.7

This logical grouping is a natural fit for a nested, compositional data structure, such as a Rust struct where fields are themselves other structs (e.g., struct Config { system\_identity: IdentityConfig, software\_management: SoftwareConfig,... }). The current flat structure of cloud-init's YAML is a primary source of its complexity. Reorganizing the configuration schema to mirror this functional taxonomy is a core principle of the design proposed in Part II. The following table provides a comprehensive inventory and analysis of the available modules, bridging the gap between the existing system and the proposed unified schema.  
**Table 1: Comprehensive Cloud-Init Module Taxonomy and Analysis**

| Module Name (cc\_) | Primary YAML Key(s) | Functional Description | Execution Frequency | Proposed Unified Schema Group |
| :---- | :---- | :---- | :---- | :---- |
| set\_hostname | hostname, fqdn | Sets the system's hostname and FQDN. | PER\_INSTANCE | System Identity & Environment |
| update\_hostname | update\_hostname | Updates hostname across reboots if metadata changes. | PER\_ALWAYS | System Identity & Environment |
| update\_etc\_hosts | manage\_etc\_hosts | Manages entries in /etc/hosts for the instance's hostname. | PER\_INSTANCE | System Identity & Environment |
| timezone | timezone | Sets the system's time zone. | PER\_INSTANCE | System Identity & Environment |
| locale | locale | Configures the default system locale. | PER\_INSTANCE | System Identity & Environment |
| keyboard | keyboard | Sets the default keyboard layout. | PER\_INSTANCE | System Identity & Environment |
| users\_groups | users, groups | Creates and configures system users and groups. | PER\_INSTANCE | User and Access Control |
| ssh | ssh\_keys, ssh\_pwauth | Configures SSH daemon and manages host keys. | PER\_INSTANCE | User and Access Control |
| ssh\_import\_id | ssh\_import\_id | Imports SSH public keys from services like GitHub/Launchpad. | PER\_INSTANCE | User and Access Control |
| ssh\_authkey\_fingerprints | ssh\_authorized\_keys | Manages authorized SSH keys for users. | PER\_INSTANCE | User and Access Control |
| set\_passwords | password, chpasswd | Sets user passwords and manages expiration policies. | PER\_INSTANCE | Authentication & Secrets |
| ca\_certs | ca-certs | Adds trusted CA certificates to the system trust store. | PER\_INSTANCE | Authentication & Secrets |
| package\_update\_upgrade\_install | package\_update, package\_upgrade, packages | Updates package lists, upgrades packages, and installs new packages. | PER\_INSTANCE | Software Provisioning |
| apt\_configure | apt | Configures APT sources, proxies, and other settings. | PER\_INSTANCE | Software Provisioning |
| yum\_add\_repo | yum\_repos | Adds YUM repository configurations. | PER\_INSTANCE | Software Provisioning |
| apk\_configure | apk\_repos | Configures APK repositories for Alpine Linux. | PER\_INSTANCE | Software Provisioning |
| snap | snaps | Installs and configures Snap packages. | PER\_INSTANCE | Software Provisioning |
| rh\_subscription | rh\_subscription | Manages Red Hat Subscription Manager registration. | PER\_INSTANCE | Software Provisioning |
| disk\_setup | disk\_setup | Defines partition layouts for storage devices. | PER\_INSTANCE | Storage & Filesystem |
| fs\_setup | fs\_setup | Creates filesystems on specified devices. | PER\_INSTANCE | Storage & Filesystem |
| growpart | growpart | Resizes a partition to fill available space on a block device. | PER\_ALWAYS | Storage & Filesystem |
| resizefs | resizefs | Resizes a filesystem to match the size of its underlying partition. | PER\_INSTANCE | Storage & Filesystem |
| mounts | mounts, swap | Configures /etc/fstab entries for mounts and swap. | PER\_INSTANCE | Storage & Filesystem |
| resolv\_conf | manage\_resolv\_conf | Manages the content of /etc/resolv.conf. | PER\_INSTANCE | Network Configuration |
| fan | fan | Configures Ubuntu Fan networking. | PER\_INSTANCE | Network Configuration |
| write\_files | write\_files | Writes arbitrary files to the filesystem. | PER\_INSTANCE | Content & File Management |
| bootcmd | bootcmd | Executes shell commands early in the boot process. | PER\_INSTANCE | Imperative Execution Hooks |
| runcmd | runcmd | Executes shell commands late in the boot process. | PER\_INSTANCE | Imperative Execution Hooks |
| scripts\_per\_boot | scripts-per-boot | Runs scripts from a directory on every boot. | PER\_ALWAYS | Imperative Execution Hooks |
| scripts\_per\_instance | scripts-per-instance | Runs scripts from a directory once per instance. | PER\_INSTANCE | Imperative Execution Hooks |
| ansible | ansible | Installs Ansible and runs playbooks via ansible-pull. | PER\_INSTANCE | Third-Party Integrations |
| puppet | puppet | Installs and configures the Puppet agent. | PER\_INSTANCE | Third-Party Integrations |
| chef | chef | Installs and configures the Chef client. | PER\_INSTANCE | Third-Party Integrations |
| salt\_minion | salt-minion | Installs and configures the Salt Minion. | PER\_INSTANCE | Third-Party Integrations |
| lxd | lxd | Initializes and configures LXD. | PER\_INSTANCE | Third-Party Integrations |
| power\_state\_change | power\_state | Changes the system power state (reboot, poweroff). | PER\_INSTANCE | System Control |

### **Section 1.3: Analysis of Design Inconsistencies and Redundancies**

The organic, module-driven evolution of cloud-init has led to a configuration model with significant design flaws. These are not merely aesthetic issues; they create ambiguity, increase the potential for user error, and complicate the development of strongly-typed tooling. The redundancies and inconsistencies are symptoms of a core architectural decision to favor module independence over a centralized, coherent data model. Each module was developed as a semi-independent plugin, defining its own configuration schema fragment. The absence of a strong, governing data model has resulted in a fragmented and overlapping user-facing API. The solution, therefore, is not to patch these individual issues but to introduce the missing architectural component: a single, authoritative, top-down configuration schema that imposes consistency.

#### **In-Depth Case Study: The Many Ways to Set a Password**

The most egregious example of this architectural deficiency, and the one that directly prompted the user's query, is the management of user passwords. There are at least three distinct, overlapping mechanisms for setting a user's password within a cloud-config file, each with different scopes, capabilities, and limitations.

1. **Mechanism 1: The Top-Level password Key.** This is the simplest method. A top-level password key sets the password for the *default user* of the system (e.g., ubuntu on Ubuntu, cloud-user on RHEL-based systems). Its scope can be modified by the top-level user key, which redefines the default user for the entire configuration. This method is straightforward but limited to a single user and typically expects a plaintext password.13  
2. **Mechanism 2: The passwd Key within a users List Item.** When defining users within the users list, each user object can contain a passwd key. This key is used to set a password for that *specific* user. Crucially, this method is designed to accept a pre-computed password *hash*, not a plaintext password. Historically, this method has had inconsistent support for password expiration flags, and if the user already exists on the system, cloud-init may skip setting the password altogether.6  
3. **Mechanism 3: The chpasswd Module.** This is the most powerful and flexible mechanism. Activated by the chpasswd key, this module can set passwords for *any* number of users, including the root user, via a multi-line list or a more structured users array. It supports both plaintext and (in more recent versions) hashed passwords, and it provides a global expire flag to force a password change on first login for all affected users.13

The coexistence of these three mechanisms creates a high degree of ambiguity. A user can define a password for the same user in multiple places. For example, if a configuration specifies a password for the ubuntu user via the top-level password key, defines a different hashed password via the passwd key in the users list entry for ubuntu, and sets a third password in the chpasswd list, the final state of the system is not immediately obvious from a plain reading of the configuration. This lack of orthogonality violates the principle of a single source of truth and makes the configuration difficult to reason about and debug.  
**Table 2: Comparative Analysis of Cloud-Init Password Directives**

| Directive | Scope | Target User(s) | Supported Formats | Expiration Control | Key Snippet References |
| :---- | :---- | :---- | :---- | :---- | :---- |
| password: \<pwd\> | Global (Default User) | The single default user (e.g., ubuntu) or user specified by user:. | Plaintext | Global (via chpasswd: {expire: true}) | 13 |
| users: \- passwd: \<hash\> | Per-User | A specific user defined within the users list. | Hashed | Inconsistent; historically none. | 6 |
| chpasswd: list: or users: | Global (Multi-User) | Any user, including root and users from the users list. | Plaintext, Hashed, Random | Global (expire: true/false) | 13 |

#### **Identifying Other Anti-Patterns**

The issues with password management are not an isolated case. Similar patterns of redundancy and inconsistency appear in other parts of the cloud-init configuration model.

* **APT Proxy Configuration:** The configuration of an APT proxy for Debian-based systems can be achieved in at least four different ways within the apt module. The module provides convenient top-level keys: proxy (an alias for http\_proxy), http\_proxy, https\_proxy, and ftp\_proxy. Alternatively, a user can provide a multi-line conf string and write the raw APT configuration lines directly, such as Acquire::http::Proxy "http://proxy.example.com:8080";.15 The convenience keys were likely added to simplify a common task, but their coexistence with the generic  
  conf method creates multiple paths to the same outcome, violating the principle of orthogonality.  
* **Inconsistent Data Structures:** Different modules often expect data for similar concepts in slightly different formats, increasing the cognitive load on the user and complicating the creation of strongly-typed schemas.  
  * The packages key can be a simple list of package name strings (e.g., \- git). To specify a particular version, the format must change to a list of two-element lists (e.g., \- \[libpython2.7, 2.7.3-0ubuntu3.1\]).12  
  * The sudo key within a user definition can be a single string representing one sudo rule (e.g., sudo: "ALL=(ALL) NOPASSWD:ALL") or a list of strings for multiple rules.10

This type of polymorphic structure, while flexible in YAML and easy to parse in a dynamically typed language like Python, is an anti-pattern for a configuration language intended to be modeled by strongly-typed systems. It forces the developer of a tool in a language like Rust to implement complex deserialization logic using enums (e.g., enum Package { Simple(String), Versioned(String, String) }). This pushes complexity from the configuration tool into the user's domain, which is precisely the opposite of what a well-designed abstraction should do.

## **Part II: A Unified Configuration Model for Cloud Initialization**

The preceding analysis demonstrates a clear need for a new configuration schema for cloud instance initialization—one that is designed from first principles to be unified, orthogonal, and developer-centric. This section transitions from analysis to prescription, proposing a new, strongly-typed configuration model. This model is designed to be expressed intuitively in a format like KDL (KDL Document Language) and to map directly to robust data structures in a language like Rust, while still being translatable to the legacy cloud-init YAML format for backward compatibility.

### **Section 2.1: Guiding Principles for the New Schema**

The design of the new schema is guided by a set of core architectural principles intended to directly address the deficiencies identified in Part I.

* **Orthogonality:** *One concept, one configuration path.* The schema will eliminate all functional overlaps and ambiguities. There will be one, and only one, way to specify a user's password, configure a package repository proxy, or define a sudo rule. This principle ensures that the configuration is the single source of truth and is easy to reason about.  
* **Clarity and Explicitness:** *The schema will be self-documenting.* Field names will be descriptive and unambiguous, avoiding cryptic abbreviations. Implicit behaviors and "magic" string values will be replaced with explicit configuration options, such as boolean flags or structured enums. For example, instead of a sudo key that accepts a string or a list, there will be an explicit structure that differentiates between unrestricted access and a custom list of rules.  
* **Type Safety and Structurability:** *The schema will map directly to strongly-typed language constructs.* This is a primary goal to support the user's implementation in Rust. The design will avoid polymorphic list types, inconsistent data structures, and other patterns that are difficult to model with structs and enums. The schema will be strictly hierarchical and well-defined.  
* **Composition over Proliferation:** *The configuration will be structured by domain.* Instead of a flat list of over 60 top-level keys, the new model will be composed of a small number of high-level domain objects (e.g., system, users, storage, software). These objects will, in turn, be composed of more specific configuration structures. This approach aligns with the functional taxonomy developed in Section 1.2 and makes the configuration more navigable and modular.

### **Section 2.2: Proposed Root Configuration Structure**

Applying the principle of composition, the new model replaces the flat namespace of cloud-init with a clean, hierarchical, and domain-driven structure. This immediately simplifies the top-level view of the configuration and provides a logical entry point for each area of instance management.

#### **Proposed Rust Root Struct**

The top-level configuration can be conceptually represented by the following Rust struct. Each field corresponds to a major functional domain identified in the taxonomy.

Rust

/// The root configuration for a cloud instance.  
struct UnifiedConfig {  
    /// Configuration related to the system's identity and environment.  
    system: Option\<SystemConfig\>,  
    /// Configuration for storage devices, partitions, and filesystems.  
    storage: Option\<StorageConfig\>,  
    /// Configuration for network services like DNS.  
    networking: Option\<NetworkingConfig\>,  
    /// Configuration for software packages, repositories, and updates.  
    software: Option\<SoftwareConfig\>,  
    /// A list of user accounts to create and configure.  
    users: Vec\<UserConfig\>,  
    /// Configuration for running imperative scripts at various boot stages.  
    scripts: Option\<ScriptConfig\>,  
    /// Configuration for bootstrapping third-party tools like Ansible or Puppet.  
    integrations: Option\<IntegrationConfig\>,  
    /// Defines the final power state of the machine after initialization.  
    power\_state: Option\<PowerStateConfig\>,  
}

#### **Illustrative KDL Example**

This Rust structure maps cleanly to a KDL representation, which is more readable and less prone to the syntactic ambiguities of YAML.

Code snippet

// Top-level nodes correspond to the fields in the UnifiedConfig struct.

system {  
    hostname "web-server-01"  
    fqdn "web-server-01.example.com"  
    timezone "Etc/UTC"  
    locale "en\_US.UTF-8"  
}

software {  
    //... detailed software configuration goes here  
}

// The 'users' node contains one or more 'user' child nodes.  
users {  
    user "admin" {  
        //... detailed configuration for the 'admin' user  
    }  
    user "deployer" {  
        //... detailed configuration for the 'deployer' user  
    }  
}

This structure immediately demonstrates the benefits of the compositional approach. The dozens of unrelated top-level keys from cloud-init are replaced by a small, predictable set of logical domains. This model is inherently more intuitive for a developer to learn and use, and it provides a solid, type-safe foundation for the Rust implementation.

### **Section 2.3: Detailed Schema Design by Functional Domain**

This section provides the detailed design for the new schema, broken down by the functional domains established in the root structure. For each domain, a conceptual Rust struct is presented alongside its corresponding KDL representation, with a rationale explaining how the design addresses the specific problems identified in Part I.

#### **A Singular User and Authentication Model**

This new model is designed to be the definitive solution to the password and user management redundancy of cloud-init. It provides a single, unambiguous structure for defining a user and all of their associated properties, including authentication.  
**Problem Solved:** Eliminates the three overlapping and conflicting methods for setting user passwords and clarifies the application of sudo rules.  
**Proposed Rust Structs:**

Rust

/// Defines a single user account and all its properties.  
struct UserConfig {  
    /// The username of the account (e.g., "admin").  
    name: String,  
    /// The user's full name or description (maps to GECOS field).  
    description: Option\<String\>,  
    /// The user's default login shell (e.g., "/bin/bash").  
    shell: Option\<String\>,  
    /// A list of supplementary groups to which the user belongs.  
    groups: Vec\<String\>,  
    /// The user's primary group. If not set, a group with the same name as the user is typically created.  
    primary\_group: Option\<String\>,  
    /// Specifies whether this is a system account (often with no home directory).  
    system\_user: bool,  
    /// Overrides the default home directory path.  
    home\_directory: Option\<String\>,  
    /// Defines the user's sudo privileges.  
    sudo: Option\<SudoConfig\>,  
    /// Contains all authentication-related settings for the user.  
    authentication: AuthenticationConfig,  
}

/// An enum representing the user's sudo privilege level.  
enum SudoConfig {  
    /// The user is explicitly denied sudo access.  
    Deny,  
    /// The user is granted passwordless, unrestricted sudo access.  
    Unrestricted,  
    /// The user is granted sudo access according to a list of custom rules.  
    Custom(Vec\<String\>),  
}

/// A unified structure for all user authentication methods.  
struct AuthenticationConfig {  
    /// Configuration for the user's password.  
    password: Option\<PasswordConfig\>,  
    /// A list of full SSH public key strings to add to the user's authorized\_keys file.  
    ssh\_keys: Vec\<String\>,  
    /// A list of IDs to import from services like GitHub ("gh:username") or Launchpad ("lp:username").  
    ssh\_import\_ids: Vec\<String\>,  
}

/// Defines a user's password, enforcing security best practices.  
struct PasswordConfig {  
    /// The pre-computed password hash string (e.g., in SHA-512 crypt format).  
    hash: String,  
    /// If true, the user will be forced to change their password on their first login.  
    expire\_on\_first\_login: bool,  
}

**Proposed KDL Representation:**

Code snippet

user "admin" {  
    description "Administrator Account"  
    shell "/bin/bash"  
    groups "sudo", "docker"  
    sudo "unrestricted" // Can also be 'deny' or a list of custom rules.  
    authentication {  
        password hash="$6$rounds=4096$..." expire\_on\_first\_login=true  
        ssh\_keys "ssh-rsa AAAAB3NzaC1yc2E... user@host"  
        ssh\_import\_ids "gh:my-github-user", "lp:my-launchpad-user"  
    }  
}

user "app-runner" {  
    system\_user true  
    sudo "deny"  
    authentication {  
        // No password login for this service account.  
        ssh\_keys "ssh-ed25519 AAAA..."  
    }  
}

**Rationale:** This design is strictly orthogonal. All authentication methods are co-located within a single, mandatory authentication block for each user. The password configuration is unambiguous: it requires a pre-computed hash, promoting security best practices by preventing plaintext secrets in the configuration file. The client tool responsible for generating this KDL would be responsible for hashing any user-provided plaintext passwords. The password expiration policy is an explicit boolean flag (expire\_on\_first\_login) scoped to the individual user, removing the confusion of cloud-init's global chpasswd.expire flag. The sudo configuration uses a string enum for common cases (unrestricted, deny) for clarity, while still allowing for custom rules, resolving the polymorphic string/list issue.

#### **Abstracted Package Management**

This model provides a single, unified entry point for all software management tasks, while neatly encapsulating distribution-specific configurations in a logical, nested structure.  
**Problem Solved:** Unifies the various distro-specific package modules (apt\_configure, yum\_add\_repo, apk\_configure) and consolidates redundant configuration options like APT proxies.  
**Proposed Rust Structs:**

Rust

/// Top-level configuration for system software.  
struct SoftwareConfig {  
    /// If true, runs the package manager's update command on boot.  
    update\_on\_boot: bool,  
    /// If true, runs the package manager's upgrade command on boot.  
    upgrade\_on\_boot: bool,  
    /// A list of packages to install. Version pinning is not supported at this abstract level for simplicity.  
    packages\_to\_install: Vec\<String\>,  
    /// Distribution-specific repository configurations.  
    repositories: Option\<RepositoryConfig\>,  
    /// Configuration for the Snap package manager.  
    snaps: Vec\<SnapConfig\>,  
}

/// A container for distribution-specific repository settings.  
struct RepositoryConfig {  
    apt: Option\<AptRepositoryConfig\>,  
    yum: Option\<YumRepositoryConfig\>,  
    apk: Option\<ApkRepositoryConfig\>,  
}

/// Configuration for APT (Debian/Ubuntu).  
struct AptRepositoryConfig {  
    /// The URL of the HTTP/HTTPS proxy to use for APT.  
    proxy: Option\<String\>,  
    /// A list of PPA identifiers to add (e.g., "ppa:deadsnakes/ppa").  
    ppas: Vec\<String\>,  
    /// A list of custom APT source repositories.  
    sources: Vec\<AptSource\>,  
}

/// Defines a single custom APT source.  
struct AptSource {  
    name: String, // Used for the filename in /etc/apt/sources.list.d/  
    uri: String,  
    suites: Vec\<String\>,  
    components: Vec\<String\>,  
    key\_id: Option\<String\>, // GPG key ID from a keyserver  
    key\_server: Option\<String\>,  
}

/// Configuration for Snap packages.  
struct SnapConfig {  
    name: String,  
    channel: Option\<String\>,  
    classic: bool, // for \--classic confinement  
}

**Proposed KDL Representation:**

Code snippet

software {  
    update\_on\_boot true  
    upgrade\_on\_boot true  
    packages\_to\_install "git", "curl", "htop", "nginx"  
      
    repositories {  
        apt {  
            proxy "http://proxy.internal.example.com:8080"  
            ppas "ppa:deadsnakes/ppa"  
              
            source name="docker" {  
                uri "https://download.docker.com/linux/ubuntu"  
                suites "focal"  
                components "stable"  
                key\_id "9DC858229FC7DD38854AE2D88D81803C0EBFCD88"  
                key\_server "keyserver.ubuntu.com"  
            }  
        }  
    }

    snaps {  
        snap "certbot" classic=true  
        snap "code" channel="stable"  
    }  
}

**Rationale:** This structure provides a single, intuitive software block for all package-related tasks. Common, distribution-agnostic actions like updating and installing packages are defined at the top level. Distribution-specific settings for repository management are nested within a repositories block. This design prevents a user from attempting to configure YUM on an Ubuntu system at the schema level, providing early validation. It also resolves the APT proxy redundancy by providing a single, unambiguous proxy field within the apt configuration block.  
*(This detailed design pattern would be repeated for all other functional domains, including Storage, Networking, Scripts, and Integrations, each time referencing the modules from the taxonomy in Section 1.2 and directly addressing the inconsistencies identified in Section 1.3.)*

### **Section 2.4: Mapping the Unified Model to Cloud-Init Directives**

A new configuration schema is only useful if it can be translated into the format understood by the target system. This section provides the critical mapping logic required for a tool to convert the proposed unified KDL/Rust model into a valid cloud-init \#cloud-config YAML file. This serves as a functional specification for the implementation of the Rust library.

#### **Translation Logic and Leaky Abstractions**

The translation process involves traversing the unified configuration structure and emitting the corresponding legacy cloud-init directives. However, due to the architectural limitations of cloud-init, this translation will not always be a perfect one-to-one mapping. In some cases, the superior granularity of the new model cannot be perfectly represented, resulting in "leaky abstractions." It is crucial to document these compromises transparently.  
A prime example is the expire\_on\_first\_login flag in the PasswordConfig struct. The new schema defines this as a per-user setting, which is the architecturally correct approach. However, the analysis in Part I revealed that cloud-init's password expiration mechanism (chpasswd: {expire: true}) is a *global* setting that affects all users for whom passwords are being set.14 A direct, per-user mapping is impossible.  
Therefore, the translation logic must adopt a "best effort" approach. The rule would be: *If expire\_on\_first\_login is set to true for any user in the users list, then the generated cloud-config YAML will include the global chpasswd: {expire: true} directive.* This means that if one user is configured to require a password change, all users will be forced to do so. This is a leaky abstraction, but the new model is still superior because it makes the user's *intent* clear and explicit at the configuration level, even if the underlying tool has limitations. The generating tool should emit a warning in this scenario to inform the user of the global side effect.  
The following table provides a concrete, end-to-end example of this translation, serving as both a "Rosetta Stone" for users and a set of acceptance tests for the implementation.  
**Table 3: Unified Schema to Cloud-Init YAML Mapping**

| Unified KDL Configuration | Generated cloud-init YAML |
| :---- | :---- |
| kdl\<br/\>system {\<br/\> hostname "prod-db-01"\<br/\> timezone "Etc/UTC"\<br/\>}\<br/\>\<br/\>software {\<br/\> update\_on\_boot true\<br/\> upgrade\_on\_boot true\<br/\> packages\_to\_install "postgresql-14", "ufw"\<br/\>}\<br/\>\<br/\>users {\<br/\> user "dba" {\<br/\> description "Database Administrator"\<br/\> shell "/bin/bash"\<br/\> groups "sudo"\<br/\> sudo "unrestricted"\<br/\> authentication {\<br/\> password hash="$6$..." expire\_on\_first\_login=true\<br/\> ssh\_keys "ssh-rsa AAA... dba@corp"\<br/\> }\<br/\> }\<br/\> user "postgres" {\<br/\> system\_user true\<br/\> sudo "deny"\<br/\> authentication {\<br/\> // No password or SSH access for service account\<br/\> }\<br/\> }\<br/\>}\<br/\>\<br/\>scripts {\<br/\> run\_commands {\<br/\> command "ufw allow 5432/tcp"\<br/\> command "ufw enable"\<br/\> }\<br/\>} | yaml\<br/\>\#cloud-config\<br/\>hostname: prod-db-01\<br/\>manage\_etc\_hosts: true\<br/\>timezone: Etc/UTC\<br/\>package\_update: true\<br/\>package\_upgrade: true\<br/\>packages:\<br/\> \- postgresql-14\<br/\> \- ufw\<br/\>users:\<br/\> \- name: dba\<br/\> gecos: Database Administrator\<br/\> shell: /bin/bash\<br/\> groups: sudo\<br/\> sudo: "ALL=(ALL) NOPASSWD:ALL"\<br/\> passwd: "$6$..."\<br/\> ssh\_authorized\_keys:\<br/\> \- "ssh-rsa AAA... dba@corp"\<br/\> \- name: postgres\<br/\> system: true\<br/\>chpasswd:\<br/\> expire: true \# Global flag set due to 'dba' user setting\<br/\>runcmd:\<br/\> \- "ufw allow 5432/tcp"\<br/\> \- "ufw enable" |

## **Part III: Conclusion and Strategic Recommendations**

This report has conducted a comprehensive deconstruction of the cloud-init configuration model and, from that analysis, has prescribed a new, unified schema designed for clarity, safety, and developer productivity. The proposed model directly addresses the architectural deficiencies of the legacy system by embracing principles of orthogonality and composition. This final section summarizes the architectural benefits of the new model and provides strategic recommendations for its implementation.

### **Section 3.1: Summary of Architectural Benefits**

The proposed unified configuration model successfully achieves the primary goals set forth in the initial query, offering a significant improvement over the existing cloud-init YAML format.

* **Unified & Simplified:** The new schema replaces the flat, unstructured list of over 60 cloud-init keys with approximately seven to eight logical, nested domains. This compositional structure, based on the functional taxonomy, dramatically reduces the cognitive load required to write and understand an instance configuration.  
* **Orthogonal:** The design systematically eliminates redundant and conflicting directives. The singular, well-defined authentication block within each user configuration is the clearest example, resolving the ambiguity of the three separate password-setting mechanisms in cloud-init. This ensures that the configuration has a single source of truth for every aspect of the system's state.  
* **Type-Safe:** The schema is designed with a direct mapping to strongly-typed language constructs, specifically Rust structs and enums. By avoiding polymorphic types and enforcing a strict, hierarchical structure, the model provides a solid foundation for building a robust, type-safe library that can validate configurations before they are ever deployed.  
* **Developer-Friendly:** The KDL representation is more readable, less error-prone, and more self-documenting than the legacy YAML format. The clear separation of concerns and explicit naming conventions make the user's intent unambiguous, simplifying both the authoring and review processes for infrastructure-as-code.

### **Section 3.2: Implementation Considerations**

While the proposed schema provides a robust architectural blueprint, its practical implementation requires careful consideration of the cloud-init ecosystem's real-world complexities. The following recommendations should guide the development of the Rust library.

* **Managing cloud-init Version Discrepancies:** The capabilities of cloud-init vary between versions and distributions. For example, support for hashed passwords within the chpasswd module was added in a specific version.24 The Rust implementation should not assume the latest version is always available. It is recommended to either:  
  1. **Target a Baseline:** Define a minimum required cloud-init version and document this as a prerequisite.  
  2. **Allow Version Targeting:** Allow the user to specify a target cloud-init version. The library can then enable or disable certain features during YAML generation and emit warnings if an unsupported feature is used.  
* **Handling Unsupported Modules:** The unified schema intentionally omits obscure, deprecated, or highly specialized cloud-init modules to maintain simplicity. However, advanced users may still require access to them. To accommodate this, the schema should include an "escape hatch" mechanism. A raw\_yaml or custom\_config block could be added to the root structure, allowing users to inject arbitrary, raw cloud-config snippets that will be merged into the final generated YAML. This provides flexibility without compromising the clarity of the core model.  
* **Validation and Error Handling:** A key advantage of this model is the ability to perform validation early in the development lifecycle. The Rust implementation should leverage its type system to provide robust validation. For example, if the user specifies a target OS family (e.g., "debian"), the library should produce a compile-time or generation-time error if the configuration contains a yum\_repositories block. This shifts error detection from a runtime surprise on a booting VM to immediate, actionable feedback on the developer's machine, significantly improving the development workflow and reliability of deployments.

#### **Works cited**

1. cloud-init 20.1 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/20.1/](https://docs.cloud-init.io/en/20.1/)  
2. cloud-init 25.3 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/](https://cloudinit.readthedocs.io/)  
3. Chapter 2\. Introduction to cloud-init \- Red Hat Documentation, accessed on September 28, 2025, [https://docs.redhat.com/en/documentation/red\_hat\_enterprise\_linux/9/html/configuring\_and\_managing\_cloud-init\_for\_rhel\_9/introduction-to-cloud-init\_cloud-content](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/configuring_and_managing_cloud-init_for_rhel_9/introduction-to-cloud-init_cloud-content)  
4. Chapter 2\. Introduction to cloud-init | Configuring and managing cloud-init for RHEL | Red Hat Enterprise Linux | 10, accessed on September 28, 2025, [https://docs.redhat.com/en/documentation/red\_hat\_enterprise\_linux/10/html/configuring\_and\_managing\_cloud-init\_for\_rhel/introduction-to-cloud-init](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/10/html/configuring_and_managing_cloud-init_for_rhel/introduction-to-cloud-init)  
5. Chapter 2\. Introduction to cloud-init \- Red Hat Documentation, accessed on September 28, 2025, [https://docs.redhat.com/en/documentation/red\_hat\_enterprise\_linux/8/html/configuring\_and\_managing\_cloud-init\_for\_rhel\_8/introduction-to-cloud-init\_cloud-content](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/8/html/configuring_and_managing_cloud-init_for_rhel_8/introduction-to-cloud-init_cloud-content)  
6. An Introduction to Cloud-Config Scripting \- DigitalOcean, accessed on September 28, 2025, [https://www.digitalocean.com/community/tutorials/an-introduction-to-cloud-config-scripting](https://www.digitalocean.com/community/tutorials/an-introduction-to-cloud-config-scripting)  
7. Index — cloud-init 22.1 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/22.1/genindex.html](https://docs.cloud-init.io/en/22.1/genindex.html)  
8. How to use cloud-init \- Ubuntu documentation, accessed on September 28, 2025, [https://documentation.ubuntu.com/lxd/latest/cloud-init/](https://documentation.ubuntu.com/lxd/latest/cloud-init/)  
9. cloud-init support for virtual machines in Azure \- Microsoft Learn, accessed on September 28, 2025, [https://learn.microsoft.com/en-us/azure/virtual-machines/linux/using-cloud-init](https://learn.microsoft.com/en-us/azure/virtual-machines/linux/using-cloud-init)  
10. Cloud config examples — Cloud-Init 18.5 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/18.5/topics/examples.html](https://docs.cloud-init.io/en/18.5/topics/examples.html)  
11. Cloud config examples — cloud-init 20.4.1 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/en/20.4.1/topics/examples.html](https://cloudinit.readthedocs.io/en/20.4.1/topics/examples.html)  
12. Cloud config examples \- cloud-init 23.3.3 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/23.3.3/reference/examples.html](https://docs.cloud-init.io/en/23.3.3/reference/examples.html)  
13. User passwords \- cloud-init 25.2 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/en/latest/reference/yaml\_examples/set\_passwords.html](https://cloudinit.readthedocs.io/en/latest/reference/yaml_examples/set_passwords.html)  
14. Chapter 7\. Configuring authentication by using cloud-init ..., accessed on September 28, 2025, [https://docs.redhat.com/en/documentation/red\_hat\_enterprise\_linux/10/html/configuring\_and\_managing\_cloud-init\_for\_rhel/configuring-authentication-by-using-cloud-init](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/10/html/configuring_and_managing_cloud-init_for_rhel/configuring-authentication-by-using-cloud-init)  
15. Module reference \- cloud-init 25.3 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/latest/reference/modules.html](https://docs.cloud-init.io/en/latest/reference/modules.html)  
16. Modules — cloud-init 22.1 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/22.1/topics/modules.html](https://docs.cloud-init.io/en/22.1/topics/modules.html)  
17. All cloud config examples \- cloud-init 25.3 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/en/latest/reference/examples.html](https://cloudinit.readthedocs.io/en/latest/reference/examples.html)  
18. Module reference \- cloud-init 23.2.1 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/en/23.2.1/reference/modules.html](https://cloudinit.readthedocs.io/en/23.2.1/reference/modules.html)  
19. Modules — cloud-init 22.1 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/en/22.1\_a/topics/modules.html](https://cloudinit.readthedocs.io/en/22.1_a/topics/modules.html)  
20. Custom cloud-init network configuration examples please? \- Canonical MAAS | Discourse, accessed on September 28, 2025, [https://discourse.maas.io/t/custom-cloud-init-network-configuration-examples-please/7745](https://discourse.maas.io/t/custom-cloud-init-network-configuration-examples-please/7745)  
21. cloud-init: What is the execution order of cloud-config directives? \- Stack Overflow, accessed on September 28, 2025, [https://stackoverflow.com/questions/34095839/cloud-init-what-is-the-execution-order-of-cloud-config-directives](https://stackoverflow.com/questions/34095839/cloud-init-what-is-the-execution-order-of-cloud-config-directives)  
22. Module reference \- cloud-init 25.3 documentation, accessed on September 28, 2025, [https://cloudinit.readthedocs.io/en/latest/reference/modules.html](https://cloudinit.readthedocs.io/en/latest/reference/modules.html)  
23. chpasswd.expire \= true doesn't work for hashed password · Issue \#4974 · canonical/cloud-init \- GitHub, accessed on September 28, 2025, [https://github.com/canonical/cloud-init/issues/4974](https://github.com/canonical/cloud-init/issues/4974)  
24. RFE: chpasswd in cloud-init should support hashed passwords · Issue \#2649 \- GitHub, accessed on September 28, 2025, [https://github.com/canonical/cloud-init/issues/2649](https://github.com/canonical/cloud-init/issues/2649)  
25. Modules — Cloud-Init 18.3 documentation, accessed on September 28, 2025, [https://docs.cloud-init.io/en/18.3/topics/modules.html](https://docs.cloud-init.io/en/18.3/topics/modules.html)