#!/bin/sh
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 1
PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
export PATH
node "$SCRIPT_DIR/scripts/local-app.mjs" "$@"
status=$?
if [ "$status" -ne 0 ] && [ -t 0 ]; then
  printf '\n失敗しました。Returnキーで閉じます。\n'
  read -r reply
fi
exit "$status"
