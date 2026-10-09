// C interface to the TFLite-Micro interpreter. Rust and the host test call it.
#ifndef TFLM_SHIM_H_
#define TFLM_SHIM_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
  TFLM_OK = 0,
  TFLM_ERR_ALREADY_INIT = 1,
  TFLM_ERR_NOT_INIT = 2,
  TFLM_ERR_MODEL_VERSION = 3,
  TFLM_ERR_ALLOCATE = 4,
  TFLM_ERR_SIZE = 5,
  TFLM_ERR_INVOKE = 6,
};

// The model and the arena must stay valid for the life of the program. The
// model needs 16-byte alignment and the arena needs 16-byte alignment.
int32_t tflm_init(const uint8_t* model, uint8_t* arena, size_t arena_len);
int32_t tflm_set_input(const int8_t* data, size_t count);
int32_t tflm_read_input(int8_t* data, size_t count);
int32_t tflm_invoke(void);
int32_t tflm_read_output(int8_t* data, size_t count);
// Bytes of the arena that the interpreter uses.
size_t tflm_arena_used(void);

#ifdef __cplusplus
}
#endif

#endif  // TFLM_SHIM_H_
