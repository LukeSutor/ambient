import os
import torch
from transformers import Qwen2_5_VLForConditionalGeneration, AutoProcessor
from qwen_vl_utils import process_vision_info
from tqdm import tqdm
import json
import re
from PIL import Image

CACHE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "../models"))
DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/images")
OUTPUT_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/generated_data.json") # Define output file path
PROMPT = """You are an expert screen activity analyzer helping create a dataset for a user productivity assistant. Your task is to generate concise, structured descriptions of user activities shown in computer screenshots. These descriptions will be embedded in a vector database to identify patterns in user behavior.

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
BATCH_SIZE = 10

def extract_json_string(text):
    """Extracts the JSON string from markdown code blocks."""
    match = re.search(r'```json\s*(\{.*?\})\s*```', text, re.DOTALL)
    if match:
        return match.group(1).strip()
    # Fallback if no markdown block found, try to find JSON directly
    match = re.search(r'(\{.*?\})', text, re.DOTALL)
    if match:
        # Basic validation to check if it looks like JSON
        potential_json = match.group(1).strip()
        if potential_json.startswith('{') and potential_json.endswith('}'):
             try:
                 # Attempt to parse to ensure it's valid JSON before returning string
                 json.loads(potential_json)
                 return potential_json
             except json.JSONDecodeError:
                 pass # Not valid JSON, fall through
    # If no JSON found or extraction failed, return original text or handle as needed
    print(f"Warning: Could not extract JSON from: {text}") # Optional warning
    # Attempt to find any JSON-like structure as a last resort, be cautious
    match = re.search(r'(\{.*?\})', text, re.DOTALL)
    if match:
        potential_json = match.group(1).strip()
        # Basic validation
        if potential_json.startswith('{') and potential_json.endswith('}'):
            try:
                json.loads(potential_json) # Check if it parses
                print(f"Warning: Found JSON-like string outside markdown: {potential_json}")
                return potential_json # Return if it looks like JSON
            except json.JSONDecodeError:
                pass # Not valid JSON
    return text # Return original text if no valid JSON found

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
    processor = AutoProcessor.from_pretrained("Qwen/Qwen2.5-VL-32B-Instruct", min_pixels=min_pixels, max_pixels=max_pixels, padding_side="left")

    # Load existing data if output file exists
    processed_filenames = set()
    results_data = []
    if os.path.exists(OUTPUT_FILE):
        try:
            with open(OUTPUT_FILE, 'r') as f:
                results_data = json.load(f)
                processed_filenames = {item['filename'] for item in results_data}
            print(f"Loaded {len(processed_filenames)} existing results from {OUTPUT_FILE}")
        except json.JSONDecodeError:
            print(f"Warning: Could not decode JSON from {OUTPUT_FILE}. Starting fresh.")
            results_data = [] # Reset if file is corrupted
            processed_filenames = set()
        except Exception as e:
            print(f"Error loading {OUTPUT_FILE}: {e}. Starting fresh.")
            results_data = [] # Reset on other errors
            processed_filenames = set()

    all_filenames = os.listdir(DATA_DIR)
    # Filter out already processed files
    filenames_to_process = [f for f in all_filenames if f not in processed_filenames]
    print(f"Found {len(all_filenames)} total files, {len(filenames_to_process)} remaining to process.")

    # Wrap the loop with tqdm for progress tracking
    for i in tqdm(range(0, len(filenames_to_process), BATCH_SIZE), desc="Processing batches"):
        batch_filenames = filenames_to_process[i:i + BATCH_SIZE]
        valid_batch_messages = []
        valid_batch_filenames = []

        for filename in batch_filenames:
            file_path = os.path.join(DATA_DIR, filename)
            try:
                # Try to open and load the image to catch truncation errors early
                img = Image.open(file_path)
                img.load() # Force loading image data to trigger potential errors

                # If loading succeeds, create messages and add to valid lists
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
                valid_batch_messages.append(messages)
                valid_batch_filenames.append(filename)
            except Exception as e:
                print(f"\nWarning: Skipping file {filename} due to unexpected error: {e}")

        # If no valid images were found in the batch, skip to the next iteration
        if not valid_batch_messages:
            print(f"\nWarning: Skipping batch starting at index {i} as no valid images were found.")
            continue

        # Prepare batch inputs using only valid messages
        texts = [
            processor.apply_chat_template(msg, tokenize=False, add_generation_prompt=True)
            for msg in valid_batch_messages # Use valid messages
        ]
        # process_vision_info might still raise errors if internal processing fails,
        # but basic file corruption should be caught above.
        try:
            image_inputs, video_inputs = process_vision_info(valid_batch_messages) # Use valid messages
        except Exception as e:
            print(f"\nError during process_vision_info for batch starting at index {i}: {e}. Skipping batch.")
            # Optionally, try to identify which image within the valid list caused the issue if possible
            continue # Skip this batch if process_vision_info fails

        inputs = processor(
            text=texts,
            images=image_inputs,
            videos=video_inputs,
            padding=True,
            return_tensors="pt",
        )
        inputs = inputs.to(device)

        # Batch inference
        try:
            generated_ids = model.generate(**inputs, max_new_tokens=512)
            generated_ids_trimmed = [
                out_ids[len(in_ids) :] for in_ids, out_ids in zip(inputs.input_ids, generated_ids)
            ]
            output_texts = processor.batch_decode(
                generated_ids_trimmed, skip_special_tokens=True, clean_up_tokenization_spaces=False
            )
        except Exception as e:
            print(f"\nError during model generation for batch starting at index {i}: {e}. Skipping batch.")
            continue # Skip this batch if generation fails

        # Store results for the current batch (using valid filenames)
        batch_results = []
        # Ensure output_texts aligns with valid_batch_filenames
        if len(valid_batch_filenames) == len(output_texts):
            for filename, output_text in zip(valid_batch_filenames, output_texts): # Use valid filenames
                extracted_json_str = extract_json_string(output_text) # Extract JSON string
                batch_results.append({"filename": filename, "generation": extracted_json_str}) # Store extracted string
        else:
            print(f"\nWarning: Mismatch between number of valid filenames ({len(valid_batch_filenames)}) and generated outputs ({len(output_texts)}) for batch starting at index {i}. Skipping result saving for this batch.")

        # Append batch results to the main list
        results_data.extend(batch_results)

        # Save results to JSON file after each batch
        try:
            os.makedirs(os.path.dirname(OUTPUT_FILE), exist_ok=True) # Ensure output directory exists
            with open(OUTPUT_FILE, 'w') as f:
                json.dump(results_data, f, indent=4)
        except Exception as e:
            print(f"\nError saving progress to {OUTPUT_FILE}: {e}")

    print(f"\nFinished processing. Total results saved: {len(results_data)}")

if __name__ == "__main__":
    main()