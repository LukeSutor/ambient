import os
import torch
from transformers import Qwen2_5_VLForConditionalGeneration, AutoProcessor
from qwen_vl_utils import process_vision_info
from tqdm import tqdm
import json
import re

CACHE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "../models"))
DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/images")
OUTPUT_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/generated_data.json") # Define output file path
PROMPT = """You are an expert screen activity analyzer helping create a dataset for a user productivity assistant. Your task is to generate structured, highly specific descriptions of user activities shown in computer screenshots. These descriptions will be embedded in a vector database to detect patterns in user behavior for intelligent recommendations.
Output Format
For each screenshot, provide a JSON object with two key fields:
{
  "application": "Specific software/application name the user is using",
  "description": "Precise, detailed description of the user's activity (10-15 words maximum)"
}
Guidelines

Be extremely specific about the application name (e.g., "Chrome", "VSCode", "Excel", "Slack")
Make descriptions highly detailed but concise - every word must contribute meaningful information
Focus on capturing actionable patterns (what the user is doing, not just what is visible)
Include relevant context that would help distinguish this activity from similar ones
Prioritize information that would be useful for pattern recognition in a vector database
Avoid generic descriptions; be precise about content, actions, and purpose
Ensure descriptions are optimized for semantic search and similarity matching

Special Cases

For browsers, try to identify the specific service (e.g., "Gmail in Chrome", "YouTube in Firefox"), as well as the specific website (e.g. "youtube.com") in the description
For development environments, note the programming language or framework when visible
For productivity tools, mention the specific type of document or project
For communication tools, distinguish between reading, writing, or other activities, as well as the names of the people being communicated with
For a blank screen (like the user being on their homepage with no applications open), indicate that they are on the homepage and are not partaking in any activity.

Examples
Example 1 - Word Processing:
{
  "application": "Microsoft Word",
  "description": "Editing business proposal with financial projections table and executive summary"
}
Example 2 - Programming:
{
  "application": "VSCode",
  "description": "Writing Python function using pandas for data cleaning in machine learning pipeline"
}
Example 3 - Web Browsing:
{
  "application": "Chrome",
  "description": "Reading AWS Lambda documentation on docs.aws.amazon.com/lambda/ focused on deployment configuration settings"
}
Example 4 - Email:
{
  "application": "Gmail",
  "description": "Composing team email about project timeline with bullet-point deliverables"
}
Example 5 - Data Analysis:
{
  "application": "Excel",
  "description": "Analyzing quarterly sales dashboard with filtered regional performance metrics"
}
Analyze the provided screenshot and generate an accurate, structured description following this format, optimized for vector embedding and similarity search."""
BATCH_SIZE = 8

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
    return text # Return original text if no JSON block found

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

    all_filenames = os.listdir(DATA_DIR)
    # Process only a subset for testing - remove [:8] to process all
    filenames_to_process = all_filenames[:64]

    results_data = [] # Initialize list to store results

    # Wrap the loop with tqdm for progress tracking
    for i in tqdm(range(0, len(filenames_to_process), BATCH_SIZE), desc="Processing batches"):
        batch_filenames = filenames_to_process[i:i + BATCH_SIZE]
        batch_messages = []

        for filename in batch_filenames:
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
            batch_messages.append(messages)

        # Prepare batch inputs
        texts = [
            processor.apply_chat_template(msg, tokenize=False, add_generation_prompt=True)
            for msg in batch_messages
        ]
        image_inputs, video_inputs = process_vision_info(batch_messages)

        inputs = processor(
            text=texts,
            images=image_inputs,
            videos=video_inputs,
            padding=True,
            return_tensors="pt",
        )
        inputs = inputs.to(device)

        # Batch inference
        generated_ids = model.generate(**inputs, max_new_tokens=512)
        generated_ids_trimmed = [
            out_ids[len(in_ids) :] for in_ids, out_ids in zip(inputs.input_ids, generated_ids)
        ]
        output_texts = processor.batch_decode(
            generated_ids_trimmed, skip_special_tokens=True, clean_up_tokenization_spaces=False
        )

        # Store results and print
        for filename, output_text in zip(batch_filenames, output_texts):
            extracted_json_str = extract_json_string(output_text) # Extract JSON string
            results_data.append({"filename": filename, "generation": extracted_json_str}) # Store extracted string

    # Save results to JSON file
    os.makedirs(os.path.dirname(OUTPUT_FILE), exist_ok=True) # Ensure output directory exists
    with open(OUTPUT_FILE, 'w') as f:
        json.dump(results_data, f, indent=4)
    print(f"\nGenerated data saved to {OUTPUT_FILE}")


if __name__ == "__main__":
    main()