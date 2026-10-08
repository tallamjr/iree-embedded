// Host test for the shim: run the model on the fixed features and compare the
// logits with the TFLite reference interpreter. Usage: host_check <model> <features>
#include <stdint.h>
#include <stdio.h>

#include "shim.h"

namespace {

constexpr size_t kFeatureCount = 49 * 40;
constexpr size_t kLogitCount = 4;
constexpr size_t kArenaBytes = 16 * 1024;
// Reference interpreter output on yes_features_int8.bin (no-softmax model):
// the pre-softmax int8 values for silence, unknown, yes, no.
constexpr int8_t kExpected[kLogitCount] = {-50, -4, 121, -4};

alignas(16) uint8_t arena[kArenaBytes];
alignas(16) uint8_t model_bytes[32 * 1024];
int8_t features[kFeatureCount];

size_t read_file(const char* path, void* buffer, size_t capacity) {
  FILE* file = fopen(path, "rb");
  if (file == nullptr) {
    perror(path);
    return 0;
  }
  size_t count = fread(buffer, 1, capacity, file);
  bool at_end = fgetc(file) == EOF;
  fclose(file);
  return at_end ? count : 0;
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 3) {
    fprintf(stderr, "usage: %s <model.tflite> <features.bin>\n", argv[0]);
    return 2;
  }
  if (read_file(argv[1], model_bytes, sizeof model_bytes) == 0) return 2;
  if (read_file(argv[2], features, sizeof features) != kFeatureCount) {
    fprintf(stderr, "features file must hold %zu bytes\n", kFeatureCount);
    return 2;
  }

  int32_t status = tflm_init(model_bytes, arena, kArenaBytes);
  if (status != TFLM_OK) {
    fprintf(stderr, "tflm_init failed: %d\n", status);
    return 1;
  }
  if ((status = tflm_set_input(features, kFeatureCount)) != TFLM_OK ||
      (status = tflm_invoke()) != TFLM_OK) {
    fprintf(stderr, "run failed: %d\n", status);
    return 1;
  }
  int8_t logits[kLogitCount];
  if ((status = tflm_read_output(logits, kLogitCount)) != TFLM_OK) {
    fprintf(stderr, "read failed: %d\n", status);
    return 1;
  }

  bool equal = true;
  for (size_t i = 0; i < kLogitCount; i++) equal = equal && logits[i] == kExpected[i];
  printf("logits %d %d %d %d, arena used %zu bytes\n", logits[0], logits[1],
         logits[2], logits[3], tflm_arena_used());
  printf(equal ? "PASS: logits equal the reference\n"
               : "FAIL: logits differ from the reference\n");
  // A second init must fail, and the shim must reject a wrong input size.
  if (tflm_init(model_bytes, arena, kArenaBytes) != TFLM_ERR_ALREADY_INIT) return 1;
  if (tflm_set_input(features, kFeatureCount - 1) != TFLM_ERR_SIZE) return 1;
  return equal ? 0 : 1;
}
