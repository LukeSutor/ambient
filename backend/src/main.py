from transformers import AutoProcessor, AutoModelForImageTextToText
import torch
import argparse
from PIL import Image
import sys

# --- Argument Parsing ---
parser = argparse.ArgumentParser(description="Analyze an image with a text prompt using SmolVLM.")
# Add positional arguments
parser.add_argument("image_path", type=str, help="Path to the image file.")
parser.add_argument("prompt", type=str, help="Text prompt for the model.")
args = parser.parse_args()

# --- Load Image ---
try:
    image = Image.open(args.image_path).convert("RGB") # Ensure image is in RGB
except FileNotFoundError:
    print(f"Error: Image file not found at {args.image_path}")
    sys.exit(1) # Use sys.exit for cleaner exit
except Exception as e:
    print(f"Error opening or processing image file: {e}")
    sys.exit(1)


dtype=torch.float16
model_path = "HuggingFaceTB/SmolVLM2-256M-Video-Instruct"
processor = AutoProcessor.from_pretrained(model_path)
model = AutoModelForImageTextToText.from_pretrained(
    model_path,
    torch_dtype=dtype,
)

messages = [
    {
        "role": "user",
        "content": [
          {"type": "text", "text": args.prompt}, # Use prompt from args
          {"type": "image", "image": image},     # Use loaded image object
        ]
    },
]

inputs = processor.apply_chat_template(
    messages,
    add_generation_prompt=True,
    tokenize=True,
    return_dict=True,
    return_tensors="pt",
).to(model.device, dtype=dtype)

generated_ids = model.generate(**inputs, do_sample=False, max_new_tokens=256)
generated_texts = processor.batch_decode(
    generated_ids,
    skip_special_tokens=True,
)
print(generated_texts[0])