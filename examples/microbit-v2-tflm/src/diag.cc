// Diagnostic hooks for diag_tflm. They expose the double-precision helpers
// that the kernels use when they prepare the requantisation multipliers.
#include <math.h>
#include <stdint.h>

#include "tensorflow/lite/micro/kernels/internal/quantization_util.h"

extern "C" {

void diag_quantize_multiplier(double real, int32_t* multiplier, int32_t* shift) {
  int shift_out = 0;
  tflite::QuantizeMultiplier(real, multiplier, &shift_out);
  *shift = shift_out;
}

double diag_frexp(double value, int32_t* exponent) {
  int exponent_out = 0;
  double fraction = frexp(value, &exponent_out);
  *exponent = exponent_out;
  return fraction;
}

double diag_round(double value) { return round(value); }

int64_t diag_double_to_i64(double value) { return static_cast<int64_t>(value); }

}  // extern "C"

extern "C" {

// Writes `pattern` over the stack below the caller. Real RAM holds old data
// there; an emulator starts with zeros. Used to find reads of uninitialised
// stack.
__attribute__((noinline)) void diag_paint_stack(uint8_t pattern, uint32_t bytes) {
  volatile uint8_t scratch[8192];
  for (uint32_t i = 0; i < bytes && i < sizeof scratch; i++) scratch[i] = pattern;
}

}  // extern "C"
