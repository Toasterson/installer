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
VARIANT=iso

UFS=install
NAME=installer
ISO_TYPE=
CONSOLE=$SERIAL
EXTRA=
OUTNAME=install

if [[ $SERIAL == vga ]]; then
	ISO_TYPE='Framebuffer Installer'
	CONSOLE=text
else
	ISO_TYPE="Serial ($SERIAL) Installer"
fi

while getopts ':N' c; do
	case "$c" in
	N)
		UFS=generic
		OUTNAME=$MACHINE
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

#
# Recreate the EFI system partition we include within the ISO file:
#
NAMEARGS=()
if [[ $ONU == yes ]]; then
	NAMEARGS+=(  )
fi
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

