#!/bin/bash
#
# Copyright 2024 Till Wegmüller
#

set -o xtrace
set -o pipefail
set -o errexit

TOP=$(cd "$(dirname "$0")" && pwd)
. "$TOP/lib/common.sh"

cd "$TOP"

#
# Build EFI Data
#
pfexec image-builder \
    build \
    -d "$DATASET" \
    -g installer \
    -n "eltorito-efi" \
    -T "$TOP/templates" \
    -N "eltorito-efi" \
    "${NAMEARGS[@]}" \
    "${ARGS[@]}"

#
# Build the ISO itself:
#

pfexec image-builder \
    build \
    -d "$DATASET" \
    -g installer \
    -n "generic-iso" \
    -T "$TOP/templates" \
    -N "generic" \
    "${NAMEARGS[@]}" \
    "${ARGS[@]}"