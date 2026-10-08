"""Remove the final SOFTMAX operator from a TFLite model.

The new model output is the int8 pre-softmax tensor. The argmax is the same,
so the softmax only adds work. Both runtimes use this cut for the int8 test.

Usage: drop_softmax.py <input.tflite> <output.tflite>
Needs the `tensorflow` and `flatbuffers` Python packages.
"""
import sys

import flatbuffers
from tensorflow.lite.python import schema_py_generated as schema

source_path, destination_path = sys.argv[1], sys.argv[2]
with open(source_path, "rb") as source_file:
    model = schema.ModelT.InitFromObj(
        schema.Model.GetRootAsModel(bytearray(source_file.read()), 0)
    )
subgraph = model.subgraphs[0]
operator_codes = [
    max(
        model.operatorCodes[op.opcodeIndex].builtinCode,
        model.operatorCodes[op.opcodeIndex].deprecatedBuiltinCode,
    )
    for op in subgraph.operators
]
assert operator_codes[-1] == schema.BuiltinOperator.SOFTMAX, operator_codes
softmax = subgraph.operators.pop()
subgraph.outputs = [softmax.inputs[0]]
builder = flatbuffers.Builder(1024)
builder.Finish(model.Pack(builder), b"TFL3")
with open(destination_path, "wb") as destination_file:
    destination_file.write(builder.Output())
print("removed softmax; new output tensor", subgraph.tensors[subgraph.outputs[0]].name.decode())
