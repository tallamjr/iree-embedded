#!/usr/bin/env bash
# Package a built runtime into a relocatable artefact:
#
#   scripts/package-runtime.sh <build-dir> <target-name> [out-dir]
#
# Layout (consumed via IREE_RUNTIME_DIR=<unpacked>/build and
# IREE_SRC_DIR=<unpacked>/src by iree-embedded-sys's build script):
#
#   iree-runtime-<target>-<shortsha>/
#     build/   archives + cmake-generated headers, original tree paths
#     src/     IREE public headers (runtime/src/**.h, flatcc include)
#     toolchains/iree_bm_config.h
#     MANIFEST.txt
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=iree-version.env
source "$ROOT/scripts/iree-version.env"

BUILD="${1:?usage: package-runtime.sh <build-dir> <target-name> [out-dir]}"
TARGET="${2:?usage: package-runtime.sh <build-dir> <target-name> [out-dir]}"
OUT="${3:-$ROOT/dist}"
SRC="${IREE_SRC:-$ROOT/.iree/src}"

NAME="iree-runtime-${TARGET}-${IREE_SHA:0:9}"
STAGE="$(mktemp -d)/$NAME"
mkdir -p "$STAGE" "$OUT"

echo "== archives + generated headers =="
# Portable copy loop. Do NOT use `install -D` here: it is GNU-only, BSD
# install fails per-file on macOS runners, and `find -exec ... \;` exits 0
# regardless, which silently produced an artefact with no archives (the
# hollow host-darwin-arm64 asset first published for v0.1.0).
(cd "$BUILD" && find . \( -name '*.a' -o \( -name '*.h' -path '*/runtime/src/*' \) \)) \
  | while read -r f; do
      mkdir -p "$STAGE/build/$(dirname "$f")"
      cp "$BUILD/$f" "$STAGE/build/$f"
    done

# Tripwire: an artefact without archives is useless to consumers; fail loudly
# rather than publish a hollow tarball.
if [ -z "$(find "$STAGE/build" -name '*.a' -print -quit)" ]; then
  echo "ERROR: no .a archives staged from $BUILD" >&2
  exit 1
fi

echo "== public headers =="
(cd "$SRC" && find runtime/src third_party/flatcc/include -name '*.h') | while read -r f; do
  mkdir -p "$STAGE/src/$(dirname "$f")"
  cp "$SRC/$f" "$STAGE/src/$f"
done

mkdir -p "$STAGE/toolchains"
cp "$ROOT/toolchains/iree_bm_config.h" "$STAGE/toolchains/"

cat > "$STAGE/MANIFEST.txt" <<EOF
target: $TARGET
iree_commit: $IREE_SHA
iree_pip_version: $IREE_PIP_VERSION (compile models with this iree-compile)
built: ${GITHUB_SHA:-local}
usage: export IREE_RUNTIME_DIR=<unpacked>/build IREE_SRC_DIR=<unpacked>/src
EOF

tar -C "$(dirname "$STAGE")" -czf "$OUT/$NAME.tar.gz" "$NAME"
echo "== DONE: $OUT/$NAME.tar.gz =="
ls -la "$OUT/$NAME.tar.gz"
