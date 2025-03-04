#!/usr/bin/bash

if ! [ -d "$HOME/image-builder" ]; then
  git clone https://github.com/illumos/image-builder "$HOME/image-builder"
fi

if ! [ -f "$HOME/.cargo/bin/image-builder" ]; then
  # shellcheck disable=SC2164
  pushd "$HOME/image-builder"
  cargo install --path .
  popd || exit
fi

TOP=$(cd "$(dirname "$0")" && pwd)
. "$TOP/lib/common.sh"

pfexec zfs create "$DATASET"