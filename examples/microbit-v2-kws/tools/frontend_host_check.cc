// Host validation harness: run the vendored TFLite-Micro front end on a WAV and
// write the 49x40 uint8 feature spectrogram, so we can confirm (by feeding it to
// the IREE model) that the on-device feature path is correct.
#include <cstdint>
#include <cstdio>
#include <cstdlib>

extern "C" {
#include "tensorflow/lite/experimental/microfrontend/lib/frontend.h"
#include "tensorflow/lite/experimental/microfrontend/lib/frontend_util.h"
}

int main(int argc, char** argv) {
  // argv[1] = input wav (16kHz mono s16), argv[2] = output features (1960 bytes)
  FILE* f = fopen(argv[1], "rb");
  fseek(f, 0, SEEK_END);
  long sz = ftell(f);
  fseek(f, 0, SEEK_SET);
  uint8_t* raw = (uint8_t*)malloc(sz);
  if (fread(raw, 1, sz, f) != (size_t)sz) return 2;
  fclose(f);
  // Skip the 44-byte canonical WAV header.
  int16_t* samples = (int16_t*)(raw + 44);
  int nsamp = (sz - 44) / 2;
  if (nsamp > 16000) nsamp = 16000;

  struct FrontendConfig config;
  FrontendFillConfigWithDefaults(&config);
  config.window.size_ms = 30;
  config.window.step_size_ms = 20;
  config.filterbank.num_channels = 40;
  config.filterbank.lower_band_limit = 125.0;
  config.filterbank.upper_band_limit = 7500.0;
  config.noise_reduction.smoothing_bits = 10;
  config.noise_reduction.even_smoothing = 0.025;
  config.noise_reduction.odd_smoothing = 0.06;
  config.noise_reduction.min_signal_remaining = 0.05;
  config.pcan_gain_control.enable_pcan = 1;
  config.pcan_gain_control.strength = 0.95;
  config.pcan_gain_control.offset = 80.0;
  config.pcan_gain_control.gain_bits = 21;
  config.log_scale.enable_log = 1;
  config.log_scale.scale_shift = 6;

  struct FrontendState state;
  if (!FrontendPopulateState(&config, &state, 16000)) {
    fprintf(stderr, "populate failed\n");
    return 3;
  }

  uint8_t feats[49 * 40] = {0};
  int frame = 0;
  size_t i = 0;
  while (i < (size_t)nsamp && frame < 49) {
    size_t num_read = 0;
    struct FrontendOutput out =
        FrontendProcessSamples(&state, samples + i, nsamp - i, &num_read);
    i += num_read;
    if (out.size > 0) {
      for (int c = 0; c < 40 && c < (int)out.size; c++) {
        int v = out.values[c];
        if (v > 255) v = 255;
        if (v < 0) v = 0;
        feats[frame * 40 + c] = (uint8_t)v;
      }
      frame++;
    }
    if (num_read == 0) break;
  }
  fprintf(stderr, "frames produced = %d\n", frame);
  FILE* of = fopen(argv[2], "wb");
  fwrite(feats, 1, sizeof(feats), of);
  fclose(of);
  return 0;
}
