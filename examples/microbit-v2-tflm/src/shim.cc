// C interface to the TFLite-Micro interpreter for the micro_speech model.
#include "shim.h"

#include <new>

#include "tensorflow/lite/micro/micro_interpreter.h"
#include "tensorflow/lite/micro/micro_mutable_op_resolver.h"

namespace {

using Resolver = tflite::MicroMutableOpResolver<3>;

// Static storage with placement new: no global constructors run at start-up.
alignas(Resolver) uint8_t resolver_storage[sizeof(Resolver)];
alignas(tflite::MicroInterpreter) uint8_t
    interpreter_storage[sizeof(tflite::MicroInterpreter)];
tflite::MicroInterpreter* interpreter = nullptr;

}  // namespace

extern "C" {

int32_t tflm_init(const uint8_t* model, uint8_t* arena, size_t arena_len) {
  if (interpreter != nullptr) return TFLM_ERR_ALREADY_INIT;
  const tflite::Model* parsed = tflite::GetModel(model);
  if (parsed->version() != TFLITE_SCHEMA_VERSION) return TFLM_ERR_MODEL_VERSION;

  Resolver* resolver = new (resolver_storage) Resolver();
  if (resolver->AddReshape() != kTfLiteOk) return TFLM_ERR_ALLOCATE;
  if (resolver->AddDepthwiseConv2D() != kTfLiteOk) return TFLM_ERR_ALLOCATE;
  if (resolver->AddFullyConnected() != kTfLiteOk) return TFLM_ERR_ALLOCATE;

  auto* candidate = new (interpreter_storage)
      tflite::MicroInterpreter(parsed, *resolver, arena, arena_len);
  if (candidate->AllocateTensors() != kTfLiteOk) return TFLM_ERR_ALLOCATE;
  interpreter = candidate;
  return TFLM_OK;
}

int32_t tflm_set_input(const int8_t* data, size_t count) {
  if (interpreter == nullptr) return TFLM_ERR_NOT_INIT;
  TfLiteTensor* input = interpreter->input(0);
  if (input->type != kTfLiteInt8 || input->bytes != count) return TFLM_ERR_SIZE;
  for (size_t i = 0; i < count; i++) input->data.int8[i] = data[i];
  return TFLM_OK;
}

int32_t tflm_invoke(void) {
  if (interpreter == nullptr) return TFLM_ERR_NOT_INIT;
  return interpreter->Invoke() == kTfLiteOk ? TFLM_OK : TFLM_ERR_INVOKE;
}

int32_t tflm_read_output(int8_t* data, size_t count) {
  if (interpreter == nullptr) return TFLM_ERR_NOT_INIT;
  const TfLiteTensor* output = interpreter->output(0);
  if (output->type != kTfLiteInt8 || output->bytes != count) return TFLM_ERR_SIZE;
  for (size_t i = 0; i < count; i++) data[i] = output->data.int8[i];
  return TFLM_OK;
}

size_t tflm_arena_used(void) {
  return interpreter == nullptr ? 0 : interpreter->arena_used_bytes();
}

}  // extern "C"
