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

// ---- Streaming mode (live mic) --------------------------------------------
// The reference micro_speech pipeline never re-windows audio: the front end
// runs continuously (noise reduction and PCAN adapt over time) and the model
// sees a rolling 49x40 spectrogram that shifts as new 20 ms frames complete.

#include <string.h>

static uint8_t g_spec[49 * 40];
static int g_spec_frames = 0; // valid trailing frames in g_spec (<= 49)

// Feed live samples into the continuously-running front end, applying DC
// removal (`mean`) and digital `gain` on the fly. Completed frames are
// appended to the rolling spectrogram. Returns frames produced by this call.
int kws_frontend_push(const int16_t* samples, int n, int32_t mean,
                      int32_t gain) {
  int16_t scratch[480];
  int produced = 0;
  int i = 0;
  while (i < n) {
    int chunk = (n - i) < 480 ? (n - i) : 480;
    for (int k = 0; k < chunk; k++) {
      int32_t v = ((int32_t)samples[i + k] - mean) * gain;
      if (v > 32767) v = 32767;
      if (v < -32768) v = -32768;
      scratch[k] = (int16_t)v;
    }
    int consumed = 0;
    while (consumed < chunk) {
      size_t num_read = 0;
      struct FrontendOutput o = FrontendProcessSamples(
          &g_state, scratch + consumed, chunk - consumed, &num_read);
      consumed += (int)num_read;
      if (o.size > 0) {
        if (g_spec_frames == 49) {
          memmove(g_spec, g_spec + 40, 48 * 40);
          g_spec_frames = 48;
        }
        uint8_t* dst = g_spec + g_spec_frames * 40;
        for (int c = 0; c < 40 && c < (int)o.size; c++) {
          int v = o.values[c];
          if (v > 255) v = 255;
          if (v < 0) v = 0;
          dst[c] = (uint8_t)v;
        }
        g_spec_frames++;
        produced++;
      }
      if (num_read == 0) break;
    }
    i += chunk;
  }
  return produced;
}

// Copy the rolling spectrogram into `out` (49*40 bytes), zero-padding the
// oldest frames while the stream warms up. Returns the valid frame count.
int kws_frontend_window(uint8_t* out) {
  int pad = 49 - g_spec_frames;
  memset(out, 0, (size_t)pad * 40);
  memcpy(out + (size_t)pad * 40, g_spec, (size_t)g_spec_frames * 40);
  return g_spec_frames;
}

// Reset the streaming state: called between the boot self-test (which runs
// unrelated audio through the shared FrontendState) and the live stream.
void kws_frontend_reset(void) {
  FrontendReset(&g_state);
  g_spec_frames = 0;
}
