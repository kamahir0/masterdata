#!/bin/sh
set -u
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 1
node "$SCRIPT_DIR/app.mjs" "$@"
status=$?
if [ "$status" -ne 0 ] && [ -t 0 ]; then
  printf '\nLocal app workflow failed (exit %s). Press Return to close.\n' "$status"
  read -r _
fi
exit "$status"
