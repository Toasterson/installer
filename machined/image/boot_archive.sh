#!/bin/bash
#
# Copyright 2024 Oxide Computer Company
# Copyright 2024 Till Wegmüller
#

set -o xtrace
set -o pipefail
set -o errexit

TOP=$(cd "$(dirname "$0")" && pwd)
. "$TOP/lib/common.sh"

# Get installer root and source common functions
INSTALLER_ROOT="$(dirname "$(dirname "$TOP")")"
source "${INSTALLER_ROOT}/lib/common.sh"

NAME=installer

cd "$TOP"

ARGS=(
	'-F' "name=$NAME"
)

#
# Build machined and place it into a place to be picked up by the image build
#
cargo build --release
MACHINED_TARGET_DIR=$(get_crate_target_dir "${INSTALLER_ROOT}/machined")
cp ${MACHINED_TARGET_DIR}/release/machined templates/files/machined

#
# Build Boot Archive
#
pfexec image-builder \
    build \
    -d "$DATASET" \
    -g installer \
    -n "generic-ttya-ufs" \
    -T "$TOP/templates" \
    -N "generic-ttya-ufs" \
    "${NAMEARGS[@]}" \
    "${ARGS[@]}"
