// Thin C shim over the TFLite-Micro audio front end, exposing two functions to
// Rust. Configured with the micro_speech settings (verified byte-exact against
// the reference TF microfrontend on the host).
#include <stddef.h>
#include <stdint.h>

#include "tensorflow/lite/experimental/microfrontend/lib/frontend.h"
#include "tensorflow/lite/experimental/microfrontend/lib/frontend_util.h"

static struct FrontendState g_state;

// Initialise the front end (allocates its state once). Returns non-zero on ok.
int kws_frontend_init(void) {
  struct FrontendConfig config;
  FrontendFillConfigWithDefaults(&config);
  config.window.size_ms = 30;
  config.window.step_size_ms = 20;
  config.filterbank.num_channels = 40;
  config.filterbank.lower_band_limit = 125.0f;
  config.filterbank.upper_band_limit = 7500.0f;
  config.noise_reduction.smoothing_bits = 10;
  config.noise_reduction.even_smoothing = 0.025f;
  config.noise_reduction.odd_smoothing = 0.06f;
  config.noise_reduction.min_signal_remaining = 0.05f;
  config.pcan_gain_control.enable_pcan = 1;
  config.pcan_gain_control.strength = 0.95f;
  config.pcan_gain_control.offset = 80.0f;
  config.pcan_gain_control.gain_bits = 21;
  config.log_scale.enable_log = 1;
  config.log_scale.scale_shift = 6;
  return FrontendPopulateState(&config, &g_state, 16000);
}

// Compute the 49x40 uint8 spectrogram from `nsamp` 16 kHz mono int16 samples.
// `out` must hold 49*40 = 1960 bytes. Returns the number of frames produced.
int kws_features(const int16_t* samples, int nsamp, uint8_t* out) {
  FrontendReset(&g_state);
  int frame = 0;
  size_t i = 0;
  while (i < (size_t)nsamp && frame < 49) {
    size_t num_read = 0;
    struct FrontendOutput o =
        FrontendProcessSamples(&g_state, (int16_t*)samples + i, nsamp - i, &num_read);
    i += num_read;
    if (o.size > 0) {
      for (int c = 0; c < 40 && c < (int)o.size; c++) {
        int v = o.values[c];
        if (v > 255) v = 255;
        if (v < 0) v = 0;
        out[frame * 40 + c] = (uint8_t)v;
      }
      frame++;
    }
    if (num_read == 0) break;
  }
  return frame;
}
