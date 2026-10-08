// Target hooks that TFLite-Micro needs. The build defines
// TF_LITE_STRIP_ERROR_STRINGS, so there is no log output. The timing code
// measures with the DWT counter in Rust, so the tick functions return 0.
#include "tensorflow/lite/micro/micro_time.h"

namespace tflite {
namespace micro {

uint32_t ticks_per_second() { return 0; }
uint32_t GetCurrentTimeTicks() { return 0; }

}  // namespace micro
}  // namespace tflite
