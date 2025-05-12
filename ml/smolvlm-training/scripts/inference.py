import os
import torch
from transformers import AutoProcessor, AutoModelForImageTextToText
from PIL import Image

# Same prompt used during training
PROMPT = """You are an expert screen activity analyzer for a user productivity assistant. Your task is to generate concise, structured descriptions of user activities shown in computer screenshots.

Output Format
For each screenshot, provide a JSON object with two key fields:
```{
  "application": "Specific application name visible in the screenshot",
  "description": "Ultra-concise description of exactly what the user is doing (include URLs for web content)"
}```

Guidelines
- Be extremely specific about the application name (e.g., "Chrome", "VSCode", "Excel", "Gmail", "Slack")
- Make descriptions extremely concise yet highly descriptive of the exact activity
- For web browsing, always include the domain (e.g., "youtube.com", "github.com", "google.com")
- If the user is on the homescreen/desktop with no active applications, explicitly state "homescreen" as the application and describe that they are not doing anything
- Focus on capturing actionable information that would help identify usage patterns
- Identify specific content being viewed or created when possible
- Mention file names, document titles, or project names if visible
- For coding, specify the programming language and project context
- Describe the exact stage of activity (reading, writing, watching, editing, etc.)

Special Cases
- If multiple windows are visible, focus on the active/forefront window
- For split-screen views, mention both visible applications
- If a video is playing, mention the content type and topic
- For communication tools, differentiate between reading, composing, or scanning messages

Examples

Example 1 - Word Processing:
```{
  "application": "Microsoft Word",
  "description": "Editing quarterly financial report with budget forecasting table highlighted"
}```

Example 2 - Programming:
```{
  "application": "VSCode",
  "description": "Writing Python data analysis function in utils.py with pandas dataframe manipulation"
}```

Example 3 - Web Browsing:
```{
  "application": "Chrome",
  "description": "Watching tutorial video on youtube.com about machine learning implementation"
}```

Example 4 - Email:
```{
  "application": "Gmail",
  "description": "Composing email to marketing team with product launch timeline attachment open"
}```

Example 5 - Data Analysis:
```{
  "application": "Excel",
  "description": "Analyzing Q3 sales data with pivot table and filtering by region"
}```

Example 6 - Desktop:
```{
  "application": "homescreen",
  "description": "Desktop visible, no active applications"
}```

Example 7 - Multiple Applications:
```{
  "application": "Zoom",
  "description": "In video meeting with 4 participants while viewing shared PowerPoint presentation about marketing strategy"
}```

Analyze the provided screenshot and generate an accurate, structured description following this format. Focus on making the description extremely specific and information-dense to optimize for vector embedding and pattern recognition."""


def generate_text_from_sample(model, processor, sample, max_new_tokens=1024, device="cuda"):
    text_input = processor.apply_chat_template(
        sample, return_dict=True, add_generation_prompt=True
    )
    image = sample[0]['content'][0]['image']
    if image.mode != 'RGB':
        image = image.convert('RGB')
    model_inputs = processor(
        text=[text_input],
        images=[[image]],
        return_tensors="pt",
    ).to(device)
    generated_ids = model.generate(**model_inputs, max_new_tokens=max_new_tokens)
    trimmed = [out_ids[len(in_ids):] for in_ids, out_ids in zip(model_inputs.input_ids, generated_ids)]
    return processor.batch_decode(trimmed, skip_special_tokens=True, clean_up_tokenization_spaces=False)[0]


def main():
    # Resolve paths relative to this script
    script_dir = os.path.dirname(__file__)
    model_dir = os.path.join(script_dir, "../results/checkpoint-1200")
    images_dir = os.path.join(script_dir, "../../../backend/sample_images")

    # Load model & processor
    device = "cuda" if torch.cuda.is_available() else "cpu"
    processor = AutoProcessor.from_pretrained("HuggingFaceTB/SmolVLM2-2.2B-Instruct")
    model = AutoModelForImageTextToText.from_pretrained(model_dir).to(device)

    # Iterate over all images in sample_images
    for fname in os.listdir(images_dir):
        if not fname.lower().endswith((".png", ".jpg", ".jpeg")):
            continue

        img_path = os.path.join(images_dir, fname)
        image = Image.open(img_path).convert("RGB")

        # Use helper functions for inference
        sample = [
            {
                "role": "user",
                "content": [
                    {"type": "image", "image": image},
                    {"type": "text",  "text": PROMPT},
                ],
            }
        ]
        result = generate_text_from_sample(model, processor, sample, max_new_tokens=256, device=device)

        print(f"--- {fname} ---")
        print(result)
        print()

if __name__ == "__main__":
    main()