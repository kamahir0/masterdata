#!/bin/sh
set -u
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPOSITORY_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
cd "$REPOSITORY_ROOT" || exit 1
cargo xtask app reinstall
status=$?
if [ "$status" -ne 0 ]; then
  printf '\nlocal app workflow failed (exit %s). Press Return to close.\n' "$status"
  read -r _
fi
exit "$status"
