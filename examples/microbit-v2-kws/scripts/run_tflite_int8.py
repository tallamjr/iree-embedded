"""Run the int8 micro_speech model in the TFLite reference interpreter.

Prints the int8 pre-softmax logits (tensor `add_1`), the softmax output, the
label and the dequantised logits for each feature file.

Usage: run_tflite_int8.py <model.tflite> <features.bin> [<features.bin> ...]
Use the full model (with softmax): the tensor `add_1` exists in it.
Needs the `tensorflow` and `numpy` Python packages.
"""
import sys
import warnings

import numpy as np

warnings.filterwarnings("ignore")
import tensorflow as tf  # noqa: E402

LABELS = ["silence", "unknown", "yes", "no"]

model_path = sys.argv[1]
interpreter = tf.lite.Interpreter(
    model_path=model_path,
    experimental_preserve_all_tensors=True,
    experimental_op_resolver_type=tf.lite.experimental.OpResolverType.BUILTIN_REF,
)
interpreter.allocate_tensors()
input_detail = interpreter.get_input_details()[0]
output_detail = interpreter.get_output_details()[0]
tensors_by_name = {detail["name"]: detail for detail in interpreter.get_tensor_details()}
pre_softmax = tensors_by_name["add_1"]
scale, zero_point = pre_softmax["quantization"]

for path in sys.argv[2:]:
    features = np.fromfile(path, dtype=np.int8).reshape(input_detail["shape"])
    interpreter.set_tensor(input_detail["index"], features)
    interpreter.invoke()
    logits = interpreter.get_tensor(pre_softmax["index"])[0]
    softmax = interpreter.get_tensor(output_detail["index"])[0]
    print(
        path.split("/")[-1],
        "pre_softmax_int8", logits.tolist(),
        "softmax_int8", softmax.tolist(),
        "label", LABELS[int(np.argmax(logits))],
        "pre_softmax_real", [round(float((int(v) - zero_point) * scale), 4) for v in logits],
    )
