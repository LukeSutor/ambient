from transformers import Qwen2_5_VLForConditionalGeneration, AutoTokenizer, AutoProcessor
from qwen_vl_utils import process_vision_info
import os
import torch

CACHE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "../models"))
DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/images")
PROMPT = """You are an image captioning expert...
"""

def main():
    device = "cuda:0" if torch.cuda.is_available() else "cpu"

    model = Qwen2_5_VLForConditionalGeneration.from_pretrained(
        "Qwen/Qwen2.5-VL-32B-Instruct",
        torch_dtype=torch.bfloat16,
        # attn_implementation="flash_attention_2",
        cache_dir=CACHE_DIR
    )
    model = model.to(device)

    min_pixels = 256*28*28
    max_pixels = 1280*28*28
    processor = AutoProcessor.from_pretrained("Qwen/Qwen2.5-VL-32B-Instruct", min_pixels=min_pixels, max_pixels=max_pixels)

    for filename in os.listdir(DATA_DIR)[:2]:
        file_path = os.path.join(DATA_DIR, filename)

        messages = [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "image": file_path,
                    },
                    {"type": "text", "text": PROMPT},
                ],
            }
        ]

        text = processor.apply_chat_template(
            messages, tokenize=False, add_generation_prompt=True
        )

        image_inputs, video_inputs = process_vision_info(messages)

        inputs = processor(
            text=[text],
            images=image_inputs,
            videos=video_inputs,
            padding=True,
            return_tensors="pt",
        )
        inputs = inputs.to(device)

        generated_ids = model.generate(**inputs, max_new_tokens=512)
        generated_ids_trimmed = [
            out_ids[len(in_ids) :] for in_ids, out_ids in zip(inputs.input_ids, generated_ids)
        ]
        output_text = processor.batch_decode(
            generated_ids_trimmed, skip_special_tokens=True, clean_up_tokenization_spaces=False
        )
        print(output_text)

if __name__ == "__main__":
    main()