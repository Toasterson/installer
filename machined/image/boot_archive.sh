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

NAME=installer

cd "$TOP"

ARGS=(
	'-F' "name=$NAME"
)

pfexec image-builder \
    build \
    -d "$DATASET" \
    -g installer \
    -n "generic-ttya-ufs" \
    -T "$TOP/templates" \
    -N "installer" \
    "${NAMEARGS[@]}" \
    "${ARGS[@]}"
