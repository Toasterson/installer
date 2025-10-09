#
# Makefile for managing the illumos-installer development VM
#
# This Makefile provides a simple, Vagrant-like interface for managing the
# development VM using libvirt and virsh. It automatically downloads the
# necessary cloud image.
#

# --- Paths and Configuration ---
# This section defines the core paths for the VM.

# Absolute path to the root of the illumos-installer repository.
ABS_REPO_PATH = $(shell pwd)

# Directory to store downloaded and generated files.
BUILD_DIR = build
ABS_BUILD_DIR = $(ABS_REPO_PATH)/$(BUILD_DIR)

# The URL for the OpenIndiana cloud image.
IMAGE_URL = https://dlc.openindiana.org/isos/hipster/20250402/OI-hipster-cloudimage.img.zstd

# The final, decompressed raw image.
VM_IMAGE = $(ABS_BUILD_DIR)/oi-hipster.img

# The downloaded, compressed image file.
IMAGE_COMPRESSED = $(ABS_BUILD_DIR)/oi-hipster.img.zstd

# Path for the UEFI NVRAM file.
NVRAM_PATH = $(ABS_BUILD_DIR)/$(VM_NAME)_VARS.fd



# --- VM Configuration ---
# Generally, you shouldn't need to change these.

# Name of the virtual machine in libvirt.
VM_NAME = illumos-installer-dev

# The template file for the VM definition.
VM_XML_TEMPLATE = examples/dev-vm-libvirt.xml

# The final, configured XML file for the VM.
VM_XML = .$(VM_NAME).xml

# Default SSH user for the VM.
SSH_USER = dev

.PHONY: all up down destroy ssh console status clean help download

all: help

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  up        - Create and start the development VM (downloads image if needed)."
	@echo "  destroy   - Stop and delete the VM definition (keeps downloaded image)."
	@echo "  ssh       - Connect to the VM using SSH."
	@echo "  console   - Connect to the VM's serial console."
	@echo "  status    - Show the current status of the VM."
	@echo "  download  - Force download of the cloud image."
	@echo "  clean     - Remove all generated files and the downloaded image."

# Main image target. This rule ensures the raw image is ready for use.
$(VM_IMAGE): $(IMAGE_COMPRESSED)
	@echo "--> Decompressing cloud image with zstd..."
	@zstd -d -o $(VM_IMAGE) $(IMAGE_COMPRESSED)

# Download target. This rule downloads the compressed image if it's missing.
$(IMAGE_COMPRESSED):
	@echo "--> Downloading OpenIndiana cloud image..."
	@mkdir -p $(ABS_BUILD_DIR)
	@curl -L -o $(IMAGE_COMPRESSED) $(IMAGE_URL)

# Alias for manually triggering the download.
download: $(IMAGE_COMPRESSED)

# Create the final VM XML by substituting the paths in the template.
$(VM_XML): $(VM_XML_TEMPLATE)
	@echo "--> Generating VM configuration for '$(VM_NAME)'..."
	@mkdir -p $(ABS_BUILD_DIR)
	@sed -e 's|/path/to/cloudimage-ttya-openindiana-hipster-dev.raw|$(VM_IMAGE)|;s|/path/to/illumos/installer|$(ABS_REPO_PATH)|;s|/path/to/nvram_vars.fd|$(NVRAM_PATH)|' \
	    $(VM_XML_TEMPLATE) > $(VM_XML)
	@echo "    VM Image Path: $(VM_IMAGE)"
	@echo "    Shared Repo Path: $(ABS_REPO_PATH)"
	@echo "    NVRAM Path: $(NVRAM_PATH)"

.PHONY: check-network
check-network:
	@echo "--> Checking for libvirt 'default' network..."
	@if ! sudo virsh net-list --all | grep -q default; then \
		echo "    ERROR: 'default' network not found, and this script cannot create it."; \
		exit 1; \
	elif sudo virsh net-list --inactive | grep -q default; then \
		echo "    'default' network is inactive. Starting it..."; \
		sudo virsh net-start default; \
	else \
		echo "    'default' network is active."; \
	fi

up: check-network $(VM_XML) $(VM_IMAGE)
	@echo "--> Creating and starting VM '$(VM_NAME)'..."
	@if sudo virsh dominfo $(VM_NAME) >/dev/null 2>&1; then \
		echo "    VM '$(VM_NAME)' already exists. Starting if not running."; \
		sudo virsh start --domain $(VM_NAME); \
	else \
		echo "    Defining domain from $(VM_XML)..."; \
		sudo virsh define $(VM_XML); \
		sudo virsh start --domain $(VM_NAME); \
	fi
	@echo "--> VM started. Use 'make ssh' or 'make console' to connect."

destroy:
	@echo "--> Destroying and undefining VM '$(VM_NAME)'..."
	@if sudo virsh dominfo $(VM_NAME) >/dev/null 2>&1; then \
		sudo virsh destroy --domain $(VM_NAME) || true; \
		sudo virsh undefine --domain $(VM_NAME) --nvram; \
	else \
		echo "    VM '$(VM_NAME)' does not exist."; \
	fi
	@rm -f $(VM_XML)

ssh:
	@echo "--> Attempting to SSH into '$(VM_NAME)' as user '$(SSH_USER)'..."
	@IP=$$(sudo virsh net-dhcp-leases default | grep $(VM_NAME) | awk '{print $$5}' | cut -d'/' -f1); \
	if [ -z "$$IP" ]; then \
		echo "    Error: Could not determine IP address for '$(VM_NAME)'."; \
		echo "    Is the VM running? Check 'make status'."; \
		exit 1; \
	fi; \
	echo "    Connecting to $$IP..."; \
	ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null $(SSH_USER)@$$IP

console:
	@echo "--> Connecting to serial console for '$(VM_NAME)'... (To exit, use Ctrl+])"
	@sudo virsh console $(VM_NAME)

status:
	@echo "--> Status for VMs managed by libvirt:"
	@sudo virsh list --all

clean:
	@echo "--> Cleaning up generated and downloaded files..."
	@rm -f $(VM_XML)
	@rm -rf $(BUILD_DIR)
