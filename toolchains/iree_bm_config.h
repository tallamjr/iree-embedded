// IREE bare-metal (generic platform) configuration, force-included into every
// translation unit of the runtime cross-build via `-include`. Kept in a header
// rather than -D flags so the macro bodies (braces, parens, spaces) don't have
// to survive shell/ninja command-line quoting.
#pragma once

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>

#define IREE_PLATFORM_GENERIC 1
#define IREE_FILE_IO_ENABLE 0

// Single-threaded synchronous target: time is monotonic-from-zero and waits are
// no-ops (synchronous inference never actually blocks).
#define IREE_TIME_NOW_FN \
  { return 0; }
#define IREE_WAIT_UNTIL_FN(nanos) (true)

// 32-bit device on Cortex-M.
#define IREE_DEVICE_SIZE_T uint32_t
#define PRIdsz PRIu32
