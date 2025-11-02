#!/bin/sh
# Wrapper script for machined that handles USB sticks
# This script checks for a USB stick, mounts it if present,
# and uses the machined binary and config from the USB if available
# This is an SMF method script that follows the guidelines in smf_method(7)

# Source SMF exit codes from standard illumos include file
. /lib/svc/share/smf_include.sh

# Check if we're running under SMF
if [ -z "$SMF_FMRI" ]; then
    echo "This script must be run as an SMF method."
    exit $SMF_EXIT_ERR_NOSMF
fi

# Log function for debugging
log_msg() {
    echo "machined-wrapper: $1" | tee -a /var/log/machined-wrapper.log
}

log_msg "Starting machined wrapper script (SMF_FMRI: $SMF_FMRI, SMF_METHOD: $SMF_METHOD)"

# Create mount point for USB
USB_MOUNT="/usb"
mkdir -p $USB_MOUNT

# Function to detect and mount USB devices
mount_usb() {
    log_msg "Searching for USB devices..."

    # Look for USB devices - this is platform specific
    # For illumos, we look for devices in /dev/dsk
    for dev in /dev/dsk/c*t*d*s0; do
        if [ -e "$dev" ]; then
            log_msg "Trying to mount $dev"

            # Try to mount as FAT32 filesystem
            mount -F pcfs $dev $USB_MOUNT 2>/dev/null
            if [ $? -eq 0 ]; then
                log_msg "Successfully mounted $dev on $USB_MOUNT"
                return 0
            fi
        fi
    done

    log_msg "No USB devices found or mounted"
    return 1
}

# Path to the system machined binary
SYSTEM_MACHINED="/usr/lib/machined"

# Try to mount USB
if mount_usb; then
    # Check if machined binary exists on USB (should be executable)
    if [ -f "$USB_MOUNT/machined" ] && [ -x "$USB_MOUNT/machined" ]; then
        log_msg "Found machined binary on USB, copying to system"
        cp "$USB_MOUNT/machined" "$SYSTEM_MACHINED"
        chmod 755 "$SYSTEM_MACHINED"
    else
        log_msg "No machined binary found on USB"
    fi

    # Check for config files with various extensions
    # The config crate automatically looks for these extensions if no format is specified
    for ext in ".conf" ".toml" ".json" ".yaml" ".yml"; do
        if [ -f "$USB_MOUNT/machined$ext" ]; then
            log_msg "Found machined$ext on USB, using it"
            # We don't need to copy it, as machined will look for it in /usb/machined
        fi
    done
else
    log_msg "No USB mounted, using system machined"
fi

# Execute machined
# Using exec to replace this process with machined
# This is important for proper contract handling by SMF
log_msg "Executing machined"
exec "$SYSTEM_MACHINED"

# If exec fails, exit with error
log_msg "Failed to execute machined"
exit $SMF_EXIT_ERR_FATAL
