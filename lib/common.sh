#!/bin/bash
# Common utilities for installer scripts

# Get the cargo target directory using cargo metadata
# Usage: get_cargo_target_dir [manifest_path]
# Returns the target directory path, or exits with error if unable to determine
get_cargo_target_dir() {
    local manifest_path="${1:-}"
    local cargo_cmd="cargo metadata --format-version 1 --no-deps"

    # Add manifest path if provided
    if [[ -n "$manifest_path" ]]; then
        cargo_cmd="$cargo_cmd --manifest-path $manifest_path"
    fi

    # Check if jq is available
    if ! command -v jq >/dev/null 2>&1; then
        echo "Error: jq is required but not installed. Please install jq." >&2
        exit 1
    fi

    # Get target directory from cargo metadata
    local target_dir
    target_dir=$($cargo_cmd 2>/dev/null | jq -r '.target_directory')

    if [[ $? -ne 0 ]] || [[ -z "$target_dir" ]] || [[ "$target_dir" == "null" ]]; then
        echo "Error: Failed to get target directory from cargo metadata" >&2
        exit 1
    fi

    echo "$target_dir"
}

# Get the cargo target directory for a specific crate
# Usage: get_crate_target_dir <crate_path>
# Returns the target directory path for the specified crate
get_crate_target_dir() {
    local crate_path="$1"

    if [[ -z "$crate_path" ]]; then
        echo "Error: crate_path is required" >&2
        exit 1
    fi

    if [[ ! -f "$crate_path/Cargo.toml" ]]; then
        echo "Error: Cargo.toml not found in $crate_path" >&2
        exit 1
    fi

    get_cargo_target_dir "$crate_path/Cargo.toml"
}

# Check if a binary exists in the target directory
# Usage: check_binary_exists <target_dir> <profile> <binary_name>
# Returns 0 if binary exists, 1 otherwise
check_binary_exists() {
    local target_dir="$1"
    local profile="$2"
    local binary_name="$3"

    if [[ -z "$target_dir" ]] || [[ -z "$profile" ]] || [[ -z "$binary_name" ]]; then
        echo "Error: target_dir, profile, and binary_name are required" >&2
        return 1
    fi

    local binary_path="$target_dir/$profile/$binary_name"
    [[ -f "$binary_path" ]]
}

# Get the path to a binary in the target directory
# Usage: get_binary_path <target_dir> <profile> <binary_name>
# Returns the full path to the binary
get_binary_path() {
    local target_dir="$1"
    local profile="$2"
    local binary_name="$3"

    if [[ -z "$target_dir" ]] || [[ -z "$profile" ]] || [[ -z "$binary_name" ]]; then
        echo "Error: target_dir, profile, and binary_name are required" >&2
        return 1
    fi

    echo "$target_dir/$profile/$binary_name"
}

# Find binary in either debug or release profile
# Usage: find_binary <target_dir> <binary_name> [preferred_profile]
# Returns the path to the binary, preferring the specified profile if available
find_binary() {
    local target_dir="$1"
    local binary_name="$2"
    local preferred_profile="${3:-release}"

    if [[ -z "$target_dir" ]] || [[ -z "$binary_name" ]]; then
        echo "Error: target_dir and binary_name are required" >&2
        return 1
    fi

    local preferred_path="$target_dir/$preferred_profile/$binary_name"
    local alternative_profile

    if [[ "$preferred_profile" == "release" ]]; then
        alternative_profile="debug"
    else
        alternative_profile="release"
    fi

    local alternative_path="$target_dir/$alternative_profile/$binary_name"

    # Check preferred profile first
    if [[ -f "$preferred_path" ]]; then
        echo "$preferred_path"
        return 0
    fi

    # Check alternative profile
    if [[ -f "$alternative_path" ]]; then
        echo "$alternative_path"
        return 0
    fi

    # Binary not found
    echo "Error: Binary '$binary_name' not found in either $preferred_profile or $alternative_profile profile" >&2
    return 1
}

# Color codes for output
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    CYAN=''
    NC=''
fi

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}
