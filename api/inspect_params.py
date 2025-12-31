from novelai.types import GenerateImageParams
import inspect

try:
    print(inspect.signature(GenerateImageParams))
    print(GenerateImageParams.model_json_schema())
except Exception as e:
    print(e)
