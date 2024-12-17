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

MACHINE=${MACHINE:-generic}
SERIAL=${SERIAL:-ttya}

UFS=install
NAME=installer
CONSOLE=$SERIAL

while getopts ':N' c; do
	case "$c" in
	N)
		UFS=generic
		;;
	\?)
		printf 'usage: %s [-o OPTE_VER] [-NO]\n' "$0" >&2
		exit 2
		;;
	esac
done

cd "$TOP"

ARGS=(
	'-F' "name=$NAME"
	'-F' "ufs=$UFS"
	'-F' "iso_type=$ISO_TYPE"
	'-F' "console=$CONSOLE"
)
if [[ $CONSOLE == tty* ]]; then
	ARGS+=( '-F' 'console_serial' )
fi

pfexec image-builder \
    build \
    -d "$DATASET" \
    -g installer \
    -n "generic-ttya-ufs" \
    -T "$TOP/templates" \
    -N "installer" \
    "${NAMEARGS[@]}" \
    "${ARGS[@]}"
