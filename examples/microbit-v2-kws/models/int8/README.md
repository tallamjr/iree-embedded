# Int8 model files

Source of the int8 test model, its no-softmax cut, and the fixed feature file. All bytes come from the upstream tflite-micro repository. No data is invented.

## Upstream files

- Repository: https://github.com/tensorflow/tflite-micro
- Pinned commit: `5328e0d852c420f9494a53135529677229dfcdd2`
- Licence: Apache-2.0 (the repository `LICENSE`).
- Training data: Speech Commands v2, CC BY (`train/README.md` in the model folder).

| File here | Upstream path | SHA-256 | Bytes |
|---|---|---|---:|
| `micro_speech_quantized.tflite` | `tensorflow/lite/micro/examples/micro_speech/models/micro_speech_quantized.tflite` | `09e5e2a9dfb2d8ed78802bf18ce297bff54281a66ca18e0c23d69ca14f822a83` | 18,800 |

URL: https://github.com/tensorflow/tflite-micro/blob/5328e0d852c420f9494a53135529677229dfcdd2/tensorflow/lite/micro/examples/micro_speech/models/micro_speech_quantized.tflite

The model input is int8 `[1,1960]`, scale 0.10171568, zero point -128. The pre-softmax tensor `add_1` is int8 `[1,4]`, scale 0.09173192, zero point 14. The labels are silence, unknown, yes, no.

## Files made from them

| File | How | SHA-256 | Bytes |
|---|---|---|---:|
| `micro_speech_int8_nosm.tflite` | `scripts/drop_softmax.py` removes the last SOFTMAX operator | `34f10a8ea3f5b75919d2c65b4a2990179cc7c04962b77e0094fd69617cb9196a` | 18,648 |
| `yes_features_int8.bin` | `scripts/gen-int8-features.sh` | `83077a32cd04b7bc28bb923faef883207fa9fad6513d2bcb526c9c3660a96c8d` | 1,960 |
| `yes_logits_tflite.txt` | `scripts/run_tflite_int8.py` on the two files above | | |

The no-softmax model is for TFLite-Micro. The IREE flow cuts the full model at `add_1_dequant` (see `scripts/compile-model.sh`), so both runtimes get the same operators.

## Feature file

`yes_features_int8.bin` holds 49 frames of 40 int8 values. The upstream front end `audio_preprocessor_int8.tflite` made them from the upstream clip `testdata/yes_1000ms.wav` (SHA-256 `b40e137a9d25bbdf135e03fa0f6bff4696d0bfc203aef8bc73b27c9428ba5a92`). TFLite-Micro ran that front end on the host, with a 30 ms window and a 20 ms stride.

To regenerate, clone the pinned commit and run:

    MAKE=gmake scripts/gen-int8-features.sh /path/to/tflite-micro

The script checks the commit and the clip hash, builds the TFLite-Micro host library, and runs `scripts/tflm_features.cc`.

The bundled Rust front end `kws-frontend` makes uint8 values, not these. It differs from the upstream front end in up to 51 of 1,960 bytes on some clips, so the int8 test does not use it.

## Reference logits

`yes_logits_tflite.txt` holds the output of the TFLite reference interpreter on the feature file. The int8 logits are -50, -4, 121, -4. The label is yes.
