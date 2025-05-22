import os
import json
from vllm import LLM
from PIL import Image
from tqdm import tqdm

CACHE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "../models"))
DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/images")
OUTPUT_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/generated_data_vllm.json")
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

def extract_json_string(text):
    import re
    try:
        match = re.search(r'```json\s*(\{.*?\})\s*```', text, re.DOTALL)
        if match:
            return match.group(1).strip()
        match = re.search(r'(\{.*?\})', text, re.DOTALL)
        if match:
            potential_json = match.group(1).strip()
            if potential_json.startswith('{') and potential_json.endswith('}'):
                import json
                try:
                    json.loads(potential_json)
                    return potential_json
                except Exception:
                    pass
        return text
    except Exception:
        return text

def main():
    # Load already processed filenames if output file exists
    processed_filenames = set()
    results_data = []
    if os.path.exists(OUTPUT_FILE):
        try:
            with open(OUTPUT_FILE, 'r') as f:
                results_data = json.load(f)
                processed_filenames = {item['filename'] for item in results_data}
        except Exception:
            results_data = []
            processed_filenames = set()

    all_filenames = os.listdir(DATA_DIR)
    filenames_to_process = [f for f in all_filenames if f not in processed_filenames and f.lower().endswith((".png", ".jpg", ".jpeg"))]

    llm = LLM(
        model="Qwen/Qwen2.5-VL-72B-Instruct-AWQ",
        download_dir="/blue/rcstudents/luke.sutor/vlm/.cache/vllm",
        max_model_len=4096  # Reduce cache size for small requests
    )

    for filename in tqdm(filenames_to_process, desc="Processing images"):
        file_path = os.path.join(DATA_DIR, filename)
        try:
            image = Image.open(file_path).convert("RGB")
        except Exception as e:
            print(f"Warning: Skipping file {filename} due to error: {e}")
            continue

        prompt_str = "<image>\n" + PROMPT
        try:
            outputs = llm.generate({
                "prompt": prompt_str,
                "multi_modal_data": {"image": image},
            })
            generated_text = outputs[0].outputs[0].text if outputs and outputs[0].outputs else ""
            extracted_json_str = extract_json_string(generated_text)
            results_data.append({"filename": filename, "generation": extracted_json_str})
            print(f"Processed: {filename}")
        except Exception as e:
            print(f"Error processing {filename}: {e}")
            continue

        # Save after each image
        try:
            os.makedirs(os.path.dirname(OUTPUT_FILE), exist_ok=True)
            with open(OUTPUT_FILE, 'w') as f:
                json.dump(results_data, f, indent=4)
        except Exception as e:
            print(f"Error saving progress to {OUTPUT_FILE}: {e}")

    print(f"Finished processing. Total results saved: {len(results_data)}")

if __name__ == "__main__":
    main()