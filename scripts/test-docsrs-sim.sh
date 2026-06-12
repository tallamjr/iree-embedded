#!/usr/bin/env bash
# Simulate the docs.rs build environment for the publishable crates.
#
# docs.rs builds in a network-isolated sandbox with DOCS_RS=1 set, and rustdoc
# never links the native runtime. This script reproduces that environment:
#   1. `cargo package` the sys crate, so the in-repo .iree/ fallback is absent
#      (a crates.io consumer's layout, which is also what docs.rs unpacks).
#   2. Build its docs with DOCS_RS=1 and all proxies pointed at an unreachable
#      address, so any attempted download fails exactly like the sandbox's
#      "could not resolve host".
#   3. Build the top-level crate's docs in-workspace under the same DOCS_RS=1
#      environment, confirming the docs path compiles for the MCU target.
#
# Dependencies are pre-fetched before the network is poisoned; only the build
# script's own network access is cut off.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="thumbv7em-none-eabihf"
sim_target_dir="${repo}/target/docsrs-sim"

# Unreachable proxy: cuts off curl (and anything else honouring proxy env
# vars) without touching the host's real network configuration.
poison=(
    env -u IREE_RUNTIME_DIR -u IREE_SRC_DIR
    DOCS_RS=1
    http_proxy=http://127.0.0.1:1
    https_proxy=http://127.0.0.1:1
    HTTP_PROXY=http://127.0.0.1:1
    HTTPS_PROXY=http://127.0.0.1:1
)

echo "==> Packaging iree-embedded-sys (crates.io layout, no .iree/)"
# --allow-dirty: the simulation packages the working tree as it stands, so it
# can validate uncommitted fixes; CI runs it on a clean checkout anyway.
cargo package -p iree-embedded-sys --no-verify --allow-dirty --quiet

version="$(grep -m1 '^version' "${repo}/crates/iree-embedded-sys/Cargo.toml" | cut -d'"' -f2)"
# The workspace may redirect its target dir (e.g. a shared CARGO_TARGET_DIR);
# ask cargo where `cargo package` actually wrote the tarball.
host_target_dir="$(cargo metadata --no-deps --format-version 1 |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
crate_file="${host_target_dir}/package/iree-embedded-sys-${version}.crate"
# Unpack outside the repo tree: inside it, cargo would adopt the extracted
# crate into this workspace, which a docs.rs build is never part of. A fresh
# directory per run, so nothing from a previous extraction lingers.
unpack_dir="$(mktemp -d "${TMPDIR:-/tmp}/iree-embedded-docsrs-sim.XXXXXX")"
trap 'rm -rf "${unpack_dir}"' EXIT
pkg_dir="${unpack_dir}/iree-embedded-sys-${version}"

# --no-verify produces only the .crate tarball; unpack it ourselves to get the
# exact tree docs.rs builds from.
tar -xzf "${crate_file}" -C "${unpack_dir}"
test -d "${pkg_dir}"

# cargo package normalises file mtimes inside the tarball, so against a reused
# target dir the freshly extracted sources look older than any previous run's
# fingerprint and cargo would rerun a stale compiled build script. Touch the
# tree so this run's sources are the newest thing cargo has seen.
find "${pkg_dir}" -type f -exec touch {} +

echo "==> Pre-fetching dependencies (network is poisoned from here on)"
(cd "${pkg_dir}" && CARGO_TARGET_DIR="${sim_target_dir}" cargo fetch --quiet)
(cd "${repo}" && cargo fetch --quiet)

echo "==> docs.rs simulation: iree-embedded-sys (packaged, offline, DOCS_RS=1)"
(cd "${pkg_dir}" && CARGO_TARGET_DIR="${sim_target_dir}" "${poison[@]}" \
    cargo doc --offline --target "${target}" --quiet)

echo "==> docs.rs simulation: iree-embedded (workspace, DOCS_RS=1)"
(cd "${repo}" && CARGO_TARGET_DIR="${sim_target_dir}" "${poison[@]}" \
    cargo doc --offline -p iree-embedded --target "${target}" --quiet)

echo "==> docs.rs simulation passed"
