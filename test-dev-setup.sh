#!/bin/bash
#
# Test script for validating the development cloud image setup
#
# This script performs various checks to ensure the development environment
# is properly configured and working as expected for all sysconfig components.
#

set -o pipefail
set -o nounset

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common functions
source "${SCRIPT_DIR}/lib/common.sh"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

test_pass() {
    log_info "✓ $1"
    ((TESTS_PASSED++))
}

test_fail() {
    log_error "✗ $1"
    ((TESTS_FAILED++))
}

test_warn() {
    log_warn "! $1"
}

print_header() {
    echo "========================================="
    echo "  Development Setup Validation Tests"
    echo "========================================="
    echo ""
}

print_summary() {
    echo ""
    echo "========================================="
    echo "  Test Summary"
    echo "========================================="
    echo "Tests passed: ${TESTS_PASSED}"
    echo "Tests failed: ${TESTS_FAILED}"
    if [[ $TESTS_FAILED -eq 0 ]]; then
        log_info "All tests passed! Development setup looks good."
        return 0
    else
        log_error "Some tests failed. Check the issues above."
        return 1
    fi
}

# Test 1: Check if we're in the right directory
test_directory_structure() {
    echo "Testing directory structure..."

    if [[ ! -f "${SCRIPT_DIR}/dev-build.sh" ]]; then
        test_fail "dev-build.sh not found in script directory"
        return
    fi

    if [[ ! -d "${SCRIPT_DIR}/sysconfig" ]]; then
        test_fail "sysconfig directory not found"
        return
    fi

    if [[ ! -d "${SCRIPT_DIR}/sysconfig-plugins" ]]; then
        test_fail "sysconfig-plugins directory not found"
        return
    fi

    if [[ ! -d "${SCRIPT_DIR}/sysconfig-provisioning" ]]; then
        test_fail "sysconfig-provisioning directory not found"
        return
    fi

    if [[ ! -d "${SCRIPT_DIR}/machined/image/templates" ]]; then
        test_fail "templates directory not found"
        return
    fi

    if [[ ! -f "${SCRIPT_DIR}/machined/image/templates/cloudimage/ttya-openindiana-hipster-dev.json" ]]; then
        test_fail "Development cloud image template not found"
        return
    fi

    if [[ ! -f "${SCRIPT_DIR}/machined/image/templates/include/sysconfig-dev.json" ]]; then
        test_fail "Sysconfig development include not found"
        return
    fi

    test_pass "Directory structure is correct"
}

# Test 2: Validate JSON syntax of templates
test_template_syntax() {
    echo "Testing template JSON syntax..."

    local dev_template="${SCRIPT_DIR}/machined/image/templates/cloudimage/ttya-openindiana-hipster-dev.json"
    local include_template="${SCRIPT_DIR}/machined/image/templates/include/sysconfig-dev.json"

    if command -v python3 >/dev/null 2>&1; then
        if python3 -m json.tool "$dev_template" >/dev/null 2>&1; then
            test_pass "Development template JSON is valid"
        else
            test_fail "Development template has invalid JSON syntax"
        fi

        if python3 -m json.tool "$include_template" >/dev/null 2>&1; then
            test_pass "Sysconfig development include JSON is valid"
        else
            test_fail "Sysconfig development include has invalid JSON syntax"
        fi
    else
        test_warn "Python3 not available - skipping JSON validation"
    fi
}

# Test 3: Check Rust toolchain
test_rust_toolchain() {
    echo "Testing Rust toolchain..."

    if ! command -v cargo >/dev/null 2>&1; then
        test_fail "Cargo not found - Rust toolchain required"
        return
    fi

    if ! command -v rustc >/dev/null 2>&1; then
        test_fail "Rust compiler not found"
        return
    fi

    test_pass "Rust toolchain is available"

    # Test if we can build all sysconfig components
    local components=("sysconfig" "sysconfig-plugins" "sysconfig-provisioning")
    for component in "${components[@]}"; do
        if [[ -f "${SCRIPT_DIR}/${component}/Cargo.toml" ]]; then
            echo "  Testing ${component} build..."
            cd "${SCRIPT_DIR}/${component}"
            if timeout 30 cargo check --quiet 2>/dev/null; then
                test_pass "${component} compiles successfully"
            else
                test_warn "${component} compilation check failed or timed out"
            fi
            cd "$SCRIPT_DIR"
        else
            test_fail "${component}/Cargo.toml not found"
        fi
    done
}

# Test 4: Check required external dependencies
test_external_deps() {
    echo "Testing external dependencies..."

    # Check for image-builder
    if [[ -f "${SCRIPT_DIR}/image-builder/Cargo.toml" ]]; then
        test_pass "image-builder source found"

        # Check if binary exists
        IMAGE_BUILDER_TARGET_DIR=$(get_crate_target_dir "${SCRIPT_DIR}/image-builder")
        if [[ -f "${IMAGE_BUILDER_TARGET_DIR}/release/image-builder" ]]; then
            test_pass "image-builder binary exists"
        else
            test_warn "image-builder binary not built (will be built automatically)"
        fi
    else
        test_fail "image-builder directory not found"
    fi

    # Check for required system commands
    local required_commands=("zfs" "zpool" "python3")
    for cmd in "${required_commands[@]}"; do
        if command -v "$cmd" >/dev/null 2>&1; then
            test_pass "$cmd command is available"
        else
            test_fail "$cmd command not found (required for build)"
        fi
    done
}

# Test 5: Check ZFS availability (if on illumos/Solaris)
test_zfs_availability() {
    echo "Testing ZFS availability..."

    if command -v zfs >/dev/null 2>&1; then
        if zfs list >/dev/null 2>&1; then
            test_pass "ZFS is functional"
        else
            test_warn "ZFS command found but may not be functional (permission issue?)"
        fi
    else
        test_warn "ZFS not available - may be required for image building"
    fi
}

# Test 6: Validate file permissions
test_file_permissions() {
    echo "Testing file permissions..."

    if [[ -x "${SCRIPT_DIR}/dev-build.sh" ]]; then
        test_pass "dev-build.sh is executable"
    else
        test_fail "dev-build.sh is not executable"
    fi

    if [[ -r "${SCRIPT_DIR}/machined/image/templates/cloudimage/ttya-openindiana-hipster-dev.json" ]]; then
        test_pass "Development template is readable"
    else
        test_fail "Development template is not readable"
    fi
}

# Test 7: Check template references
test_template_references() {
    echo "Testing template file references..."

    local template_files_dir="${SCRIPT_DIR}/machined/image/templates/files"

    # Check for files referenced in the development template
    local required_files=(
        "boot_console.ttya"
        "ttydefs.115200"
        "default_init.utc"
    )

    for file in "${required_files[@]}"; do
        if [[ -f "${template_files_dir}/${file}" ]] || [[ -f "${SCRIPT_DIR}/machined/image/templates/cloudimage/files/${file}" ]]; then
            test_pass "Template file ${file} exists"
        else
            test_fail "Template file ${file} not found"
        fi
    done

    # Check for sysconfig-dev files
    local sysconfig_dev_files=(
        "dev-9p-mount"
        "dev-9p-mount.xml"
        "sysconfig-illumos-base-plugin.xml"
        "sysconfig-provisioning.xml"
        "sysconfig.toml"
        "dev-test.kdl"
        "vfstab"
    )

    local sysconfig_dev_dir="${SCRIPT_DIR}/machined/image/templates/files/sysconfig-dev"
    for file in "${sysconfig_dev_files[@]}"; do
        if [[ -f "${sysconfig_dev_dir}/${file}" ]]; then
            test_pass "SysConfig dev file ${file} exists"
        else
            test_fail "SysConfig dev file ${file} not found"
        fi
    done

    # Check for external source files that will be needed
    local sysconfig_manifest="${SCRIPT_DIR}/sysconfig/image/templates/files/sysconfig-smf-service.xml"
    if [[ -f "$sysconfig_manifest" ]]; then
        test_pass "Sysconfig SMF manifest exists"
    else
        test_fail "Sysconfig SMF manifest not found at $sysconfig_manifest"
    fi
}

# Test 8: Check for test configuration files
test_config_files() {
    echo "Testing configuration files..."

    local config_files=(
        "sysconfig-plugins/test-provisioning-config.kdl"
        "sysconfig-plugins/test-provisioning-simple.kdl"
        "sysconfig-plugins/test-provisioning-knus.kdl"
    )

    for config in "${config_files[@]}"; do
        if [[ -f "${SCRIPT_DIR}/${config}" ]]; then
            test_pass "Test config ${config} exists"
        else
            test_warn "Test config ${config} not found (may affect testing)"
        fi
    done
}

# Test 9: Check sysconfig component binaries
test_component_binaries() {
    echo "Testing sysconfig component binaries..."

    # Get dynamic target directories
    local sysconfig_target=$(get_crate_target_dir "${SCRIPT_DIR}/sysconfig")
    local plugins_target=$(get_crate_target_dir "${SCRIPT_DIR}/sysconfig-plugins")
    local provisioning_target=$(get_crate_target_dir "${SCRIPT_DIR}/sysconfig-provisioning")

    local binaries=(
        "${sysconfig_target}/release/sysconfig"
        "${plugins_target}/release/illumos-base-plugin"
        "${provisioning_target}/release/provisioning-plugin"
    )

    for binary in "${binaries[@]}"; do
        if [[ -f "${binary}" ]]; then
            test_pass "Binary ${binary} exists"
        else
            test_warn "Binary ${binary} not built (will be built during image creation)"
        fi
    done
}

# Test 10: Validate SMF manifest syntax
test_smf_manifest_syntax() {
    echo "Testing SMF manifest syntax..."

    local manifest="${SCRIPT_DIR}/sysconfig/image/templates/files/sysconfig-smf-service.xml"

    if [[ -f "$manifest" ]]; then
        # Basic XML syntax check using xmllint if available
        if command -v xmllint >/dev/null 2>&1; then
            if xmllint --noout "$manifest" 2>/dev/null; then
                test_pass "Sysconfig SMF manifest XML is valid"
            else
                test_warn "Sysconfig SMF manifest has XML syntax issues"
            fi
        else
            # Fallback: basic check for required elements
            if grep -q "service_bundle" "$manifest" && grep -q "service.*name.*system/installer/sysconfig" "$manifest"; then
                test_pass "Sysconfig SMF manifest appears valid (basic check)"
            else
                test_warn "Sysconfig SMF manifest appears malformed (basic check)"
            fi
        fi
    else
        test_fail "Sysconfig SMF manifest not found"
    fi
}

# Test 11: Documentation completeness
test_documentation() {
    echo "Testing documentation completeness..."

    local docs=(
        "DEV_CLOUD_IMAGE.md"
        "README_DEV_SETUP.md"
        "DEVELOPMENT_GUIDE.md"
        "DEV_SETUP_SUMMARY.md"
    )

    for doc in "${docs[@]}"; do
        if [[ -f "${SCRIPT_DIR}/${doc}" ]]; then
            test_pass "Documentation file ${doc} exists"
        else
            test_warn "Documentation file ${doc} not found"
        fi
    done
}

# Test 12: Check for potential issues
test_potential_issues() {
    echo "Checking for potential issues..."

    # Check if we're running on a system that supports the virtualization needed
    if [[ -r /proc/version ]] && grep -q "Linux" /proc/version; then
        test_warn "Running on Linux - make sure QEMU/KVM supports 9P filesystem"
    fi

    # Check available disk space in /tmp (where builds might happen)
    if command -v df >/dev/null 2>&1; then
        local tmp_space
        tmp_space=$(df /tmp 2>/dev/null | awk 'NR==2 {print $4}' || echo "0")
        if [[ $tmp_space -gt 1000000 ]]; then  # > 1GB
            test_pass "Sufficient temporary space available"
        else
            test_warn "Limited temporary space - builds may fail"
        fi
    fi

    # Check for conflicting processes
    if pgrep -f "image-builder" >/dev/null 2>&1; then
        test_warn "image-builder process already running"
    fi

    # Check cargo workspace setup
    if [[ -f "${SCRIPT_DIR}/Cargo.toml" ]]; then
        test_pass "Cargo workspace configuration found"
    else
        test_warn "No Cargo workspace - components built individually"
    fi
}

# Main execution
main() {
    print_header

    test_directory_structure
    test_template_syntax
    test_rust_toolchain
    test_external_deps
    test_zfs_availability
    test_file_permissions
    test_template_references
    test_config_files
    test_component_binaries
    test_smf_manifest_syntax
    test_documentation
    test_potential_issues

    print_summary
}

# Help function
show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Test script for validating the development cloud image setup.

OPTIONS:
    -h, --help      Show this help message
    -v, --verbose   Enable verbose output (not implemented yet)

This script performs comprehensive checks to ensure the development
environment is properly configured for building and using the
development cloud image with all sysconfig components.

Tests include:
- Directory structure validation
- JSON template syntax checking
- Rust toolchain availability
- Required dependencies
- ZFS functionality
- File permissions
- Template file references
- Configuration file presence
- Component binary existence
- SMF manifest validation
- Documentation completeness
- Potential issue detection

Components tested:
- sysconfig (main daemon)
- sysconfig-plugins (plugin binaries)
- sysconfig-provisioning (oneshot CLI)
- image-builder
- SMF service manifests
- KDL configuration files

Exit codes:
    0   All tests passed
    1   One or more tests failed

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            # Verbose mode could be implemented here
            shift
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Run main function
if main; then
    exit 0
else
    exit 1
fi
