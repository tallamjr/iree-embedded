#!/usr/bin/env bash
# Fetch the pinned IREE runtime source into $IREE_SRC (default .iree/src).
# Partial clone (no blobs up front), all submodules except llvm-project,
# which only the compiler build needs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=iree-version.env
source "$ROOT/scripts/iree-version.env"
SRC="${IREE_SRC:-$ROOT/.iree/src}"

echo "== fetch IREE @ $IREE_SHA into $SRC =="
if [ ! -d "$SRC/.git" ]; then
  git clone --filter=tree:0 --no-checkout https://github.com/iree-org/iree "$SRC"
fi
git -C "$SRC" fetch origin "$IREE_SHA"
git -C "$SRC" checkout "$IREE_SHA"

echo "== submodules (all except llvm-project) =="
cd "$SRC"
paths=$(git config -f .gitmodules --get-regexp path | awk '{print $2}' | grep -v llvm-project)
# shellcheck disable=SC2086
git submodule update --init -- $paths
