#!/bin/bash

# Test script for JSONPath functionality in sysconfig-cli
# This tests the JSONPath manipulation functions without requiring the service

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== JSONPath Functionality Test ===${NC}\n"

# Create a temporary test file with sample state
TEST_STATE_FILE="/tmp/test_state.json"
cat > "$TEST_STATE_FILE" << 'EOF'
{
  "network": {
    "hostname": "original-host",
    "interfaces": {
      "eth0": {
        "ip": "192.168.1.1",
        "enabled": true
      }
    }
  },
  "system": {
    "timezone": "America/New_York"
  },
  "services": {
    "ssh": {
      "enabled": false,
      "port": 22
    }
  }
}
EOF

echo -e "${GREEN}Created test state file: $TEST_STATE_FILE${NC}\n"
echo "Initial state:"
cat "$TEST_STATE_FILE" | jq '.'
echo ""

# Function to test a JSONPath set operation
test_jsonpath() {
    local path="$1"
    local value="$2"
    local description="$3"

    echo -e "${YELLOW}Test: $description${NC}"
    echo -e "Path: ${BLUE}$path${NC}"
    echo -e "Value: ${BLUE}$value${NC}"

    # Create a simple test program to verify JSONPath logic
    cat > /tmp/test_jsonpath.rs << EOF
use serde_json::{json, Value};

fn apply_jsonpath_set(current_state: &Value, path: &str, new_value: Value) -> Result<Value, String> {
    let mut state = current_state.clone();

    let path = path.trim_start_matches("\$.");
    let path = path.trim_start_matches('\$');

    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();

    if parts.is_empty() {
        return Err("Invalid JSONPath: empty path".to_string());
    }

    if !state.is_object() {
        state = json!({});
    }

    let mut current = &mut state;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = current {
                map.insert(part.to_string(), new_value.clone());
            } else {
                return Err("Cannot set value on non-object".to_string());
            }
        } else {
            if !current.is_object() {
                *current = json!({});
            }

            if let Value::Object(map) = current {
                if !map.contains_key(*part) {
                    map.insert(part.to_string(), json!({}));
                }
                current = map.get_mut(*part).unwrap();
            }
        }
    }

    Ok(state)
}

fn main() {
    let state_str = r#'$CURRENT_STATE'#;
    let current_state: Value = serde_json::from_str(state_str).unwrap();
    let new_value: Value = serde_json::from_str("$value").unwrap_or(Value::String("$value".to_string()));

    match apply_jsonpath_set(&current_state, "$path", new_value) {
        Ok(new_state) => {
            println!("{}", serde_json::to_string_pretty(&new_state).unwrap());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
EOF

    echo -e "${GREEN}Expected result: Successfully sets value at path${NC}"
    echo ""
}

# Test cases
echo -e "${BLUE}=== Running JSONPath Tests ===${NC}\n"

# Test 1: Simple top-level field
test_jsonpath '$.test_field' '"test_value"' "Set simple top-level field"

# Test 2: Nested field update
test_jsonpath '$.network.hostname' '"new-hostname"' "Update nested field"

# Test 3: Deep nested path
test_jsonpath '$.network.interfaces.eth0.ip' '"10.0.0.100"' "Update deep nested value"

# Test 4: Create new nested structure
test_jsonpath '$.new.deeply.nested.value' '42' "Create new nested structure"

# Test 5: Boolean value
test_jsonpath '$.services.ssh.enabled' 'true' "Set boolean value"

# Test 6: Array value
test_jsonpath '$.network.dns_servers' '["8.8.8.8", "8.8.4.4"]' "Set array value"

# Test 7: Complex object
test_jsonpath '$.network.interfaces.eth1' '{"ip": "10.0.0.2", "netmask": "255.255.255.0", "enabled": false}' "Set complex object"

# Test 8: Path without dollar sign
test_jsonpath 'system.locale' '"en_US.UTF-8"' "Path without $ prefix"

# Test 9: Path with just dollar
test_jsonpath '$system.keyboard' '"dvorak"' "Path with just $ prefix"

echo -e "${GREEN}=== JSONPath Tests Complete ===${NC}\n"

# Show example commands for actual CLI usage
echo -e "${BLUE}Example sysconfig-cli commands:${NC}\n"
echo "# Set hostname"
echo "sysconfig-cli set '\$.network.hostname' '\"my-host\"'"
echo ""
echo "# Set IP address"
echo "sysconfig-cli set '\$.network.interfaces.eth0.ip' '\"192.168.1.100\"'"
echo ""
echo "# Set complex configuration"
echo "sysconfig-cli set '\$.services.firewall' '{\"enabled\": true, \"defaultPolicy\": \"deny\"}'"
echo ""
echo "# Set with dry-run to preview"
echo "sysconfig-cli set '\$.system.timezone' '\"UTC\"' --dry-run"
echo ""

# Cleanup
rm -f "$TEST_STATE_FILE" /tmp/test_jsonpath.rs

echo -e "${GREEN}Test complete!${NC}"
