from novelai.types import ControlNetImage
import json

try:
    print(json.dumps(ControlNetImage.model_json_schema(), indent=2))
except Exception as e:
    print(e)
