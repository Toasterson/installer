#!/usr/bin/bash

#
# Use the Image Builder to produce a tar file that contains an installed OpenIndiana
# system which can be used to seed an image.  The produced file should be
# something like:
#
#	/rpool/images/output/hipster-base.tar

#
# Copyright 2024 Till Wegmüller
#

TOP=$(cd "$(dirname "$0")" && pwd)
. "$TOP/lib/common.sh"

STRAP_ARGS=()
ALL_ARGS=()

while getopts 'BEf:' c; do
	case "$c" in
	f)
		#
		# Use -f to request a full reset from the image builder, thus
		# effectively destroying any existing files and starting from a
		# freshly installed set of OS files.
		#
		STRAP_ARGS+=( '--fullreset' )
		;;
	E)
		#
		# Enable OmniOS Extra (Additional packages) publisher.
		#
		ALL_ARGS+=( '-F' 'extra' )
		;;
	B)
		#
		# Install software build tools.
		#
		ALL_ARGS+=( '-F' 'build' )
		;;
	\?)
		printf 'usage: %s [-f]\n' "$0" >&2
		exit 2
		;;
	esac
done
shift $((OPTIND - 1))

cd "$TOP" || exit

for n in 01-strap 02-image 03-archive; do
  pfexec image-builder \
        build \
        -T "$TOP/templates" \
        -d "$DATASET" \
        -g "installer" \
        -n "ramdisk-${n}" \
        -F "name=installer" \
        "${ALL_ARGS[@]}" \
        "${ARGS[@]}"
done