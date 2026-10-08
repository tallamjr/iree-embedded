// Make the int8 feature file with the upstream TFLite-Micro audio front end.
// Usage: tflm_features <audio_preprocessor_int8.tflite> <audio.s16le> <out.i8>
// Runs the preprocessor model on 30 ms windows (480 samples) with a 20 ms
// stride (320 samples) until it has 49 frames of 40 int8 values.
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

#include "tensorflow/lite/micro/micro_interpreter.h"
#include "tensorflow/lite/micro/micro_log.h"
#include "tensorflow/lite/micro/micro_mutable_op_resolver.h"

namespace {

constexpr size_t kWindowSamples = 480;
constexpr size_t kStrideSamples = 320;
constexpr size_t kChannels = 40;
constexpr size_t kFrames = 49;
constexpr size_t kArenaBytes = 20000;
alignas(16) uint8_t g_arena[kArenaBytes];

std::vector<uint8_t> read_file(const char* path) {
  FILE* file = fopen(path, "rb");
  if (file == nullptr) {
    perror(path);
    exit(2);
  }
  std::vector<uint8_t> bytes;
  uint8_t chunk[4096];
  size_t count;
  while ((count = fread(chunk, 1, sizeof chunk, file)) > 0) {
    bytes.insert(bytes.end(), chunk, chunk + count);
  }
  fclose(file);
  return bytes;
}

}  // namespace

int main(int argc, char** argv) {
  if (argc != 4) {
    fprintf(stderr, "usage: %s <preprocessor.tflite> <audio.s16le> <out.i8>\n", argv[0]);
    return 2;
  }
  std::vector<uint8_t> model_bytes = read_file(argv[1]);
  std::vector<uint8_t> audio_bytes = read_file(argv[2]);
  const tflite::Model* model = tflite::GetModel(model_bytes.data());

  tflite::MicroMutableOpResolver<18> resolver;
  resolver.AddReshape();
  resolver.AddCast();
  resolver.AddStridedSlice();
  resolver.AddConcatenation();
  resolver.AddMul();
  resolver.AddAdd();
  resolver.AddDiv();
  resolver.AddMinimum();
  resolver.AddMaximum();
  resolver.AddWindow();
  resolver.AddFftAutoScale();
  resolver.AddRfft();
  resolver.AddEnergy();
  resolver.AddFilterBank();
  resolver.AddFilterBankSquareRoot();
  resolver.AddFilterBankSpectralSubtraction();
  resolver.AddPCAN();
  resolver.AddFilterBankLog();

  tflite::MicroInterpreter interpreter(model, resolver, g_arena, kArenaBytes);
  if (interpreter.AllocateTensors() != kTfLiteOk) {
    fprintf(stderr, "AllocateTensors failed\n");
    return 1;
  }

  const int16_t* samples = reinterpret_cast<const int16_t*>(audio_bytes.data());
  const size_t sample_count = audio_bytes.size() / 2;
  std::vector<int8_t> features;
  for (size_t offset = 0;
       offset + kWindowSamples <= sample_count && features.size() < kFrames * kChannels;
       offset += kStrideSamples) {
    memcpy(interpreter.input(0)->data.i16, samples + offset, kWindowSamples * sizeof(int16_t));
    if (interpreter.Invoke() != kTfLiteOk) {
      fprintf(stderr, "Invoke failed\n");
      return 1;
    }
    const int8_t* frame = interpreter.output(0)->data.int8;
    features.insert(features.end(), frame, frame + kChannels);
  }
  if (features.size() != kFrames * kChannels) {
    fprintf(stderr, "expected %zu values, got %zu\n", kFrames * kChannels, features.size());
    return 1;
  }
  FILE* out = fopen(argv[3], "wb");
  if (out == nullptr) {
    perror(argv[3]);
    return 2;
  }
  fwrite(features.data(), 1, features.size(), out);
  fclose(out);
  printf("wrote %zu bytes\n", features.size());
  return 0;
}
