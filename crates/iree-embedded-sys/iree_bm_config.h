// IREE bare-metal (generic platform) configuration, force-included into every
// translation unit of the runtime cross-build via `-include`. Kept in a header
// rather than -D flags so the macro bodies (braces, parens, spaces) don't have
// to survive shell/ninja command-line quoting.
//
// This is a committed copy of repo-root toolchains/iree_bm_config.h, carried
// inside the crate so build.rs can force-include it when compiling the MCU
// wrappers from a crates.io checkout (where the repo root is not present). The
// two files must stay identical; refresh this copy when bumping the pinned IREE
// version, alongside regenerating the committed bindings.
#pragma once

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>

#define IREE_PLATFORM_GENERIC 1
#define IREE_FILE_IO_ENABLE 0

// Keep status messages (source location + annotations) so on-device failures
// report a useful reason over RTT, not just a bare code.
#define IREE_STATUS_MODE 2

// Single-threaded synchronous target: time is monotonic-from-zero and waits are
// no-ops (synchronous inference never actually blocks).
#define IREE_TIME_NOW_FN \
  { return 0; }
#define IREE_WAIT_UNTIL_FN(nanos) (true)

// 32-bit device on Cortex-M.
#define IREE_DEVICE_SIZE_T uint32_t
#define PRIdsz PRIu32

// Ubuntu's arm-none-eabi newlib headers omit the 64-bit PRI macros (Homebrew's
// build has them). IREE format strings use them; long long is "ll" on ARM EABI.
#ifndef PRId64
#define PRId64 "lld"
#endif
#ifndef PRIi64
#define PRIi64 "lli"
#endif
#ifndef PRIu64
#define PRIu64 "llu"
#endif
#ifndef PRIx64
#define PRIx64 "llx"
#endif
#ifndef PRIX64
#define PRIX64 "llX"
#endif
