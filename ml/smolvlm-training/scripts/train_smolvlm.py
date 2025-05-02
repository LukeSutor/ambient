from transformers import AutoProcessor, AutoModelForImageTextToText, BitsAndBytesConfig
import torch
import os
from PIL import Image
from peft import LoraConfig, get_peft_model
from trl import SFTConfig, SFTTrainer
import json
import random
from dotenv import load_dotenv


CACHE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "../models"))
DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data/images")
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


# Returns a list of all data samples
def load_dataset():
    json_path = os.path.join(os.path.dirname(DATA_DIR), "generated_data.json")
    with open(json_path) as file:
        data = json.load(file)
    return data


# Create train test split
def split_dataset(data, train_proportion=0.8):
    random.shuffle(data)
    split_index = int(len(data) * train_proportion)
    train_data = data[:split_index]
    test_data = data[split_index:]
    return train_data, test_data


def format_data(sample):
    # Only store the filename, not the image object
    return [
        {
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "image_filename": sample["filename"],  # store filename
                },
                {
                    "type": "text",
                    "text": PROMPT,
                }
            ],
        },
        {
            "role": "assistant",
            "content": [{"type": "text", "text": sample["generation"]}],
        },
    ]


def generate_text_from_sample(model, processor, sample, max_new_tokens=1024, device="cuda"):
    # Prepare the text input by applying the chat template
    text_input = processor.apply_chat_template(
        sample[0],  # Use the sample without the system message
        add_generation_prompt=True
    )

    image_inputs = []
    image = sample[0]['content'][0]['image']
    if image.mode != 'RGB':
        image = image.convert('RGB')
    image_inputs.append([image])

    # Prepare the inputs for the model
    model_inputs = processor(
        text=text_input,
        images=image_inputs,
        return_tensors="pt",
    ).to(device)  # Move inputs to the specified device

    # Generate text with the model
    generated_ids = model.generate(**model_inputs, max_new_tokens=max_new_tokens)

    # Trim the generated ids to remove the input ids
    trimmed_generated_ids = [
        out_ids[len(in_ids):] for in_ids, out_ids in zip(model_inputs.input_ids, generated_ids)
    ]

    # Decode the output text
    output_text = processor.batch_decode(
        trimmed_generated_ids,
        skip_special_tokens=True,
        clean_up_tokenization_spaces=False
    )

    return output_text[0]  # Return the first decoded output text


def main():
    load_dotenv()

    # Prepare dataset
    dataset = load_dataset()
    train_dataset, eval_dataset = split_dataset(dataset)

    train_dataset = [format_data(sample) for sample in train_dataset]
    eval_dataset = [format_data(sample) for sample in eval_dataset]

    # BitsAndBytesConfig int-4 config
    bnb_config = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_use_double_quant=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.bfloat16
    )

    model_path = "HuggingFaceTB/SmolVLM2-500M-Video-Instruct"
    processor = AutoProcessor.from_pretrained(model_path)
    model = AutoModelForImageTextToText.from_pretrained(
        model_path,
        torch_dtype=torch.bfloat16,
        quantization_config=bnb_config,
        cache_dir=CACHE_DIR
    ).to("cuda")

    # Configure LoRA
    peft_config = LoraConfig(
        r=32,
        lora_alpha=64,
        lora_dropout=0.1,
        target_modules=['down_proj','o_proj','k_proj','q_proj','gate_proj','up_proj','v_proj'],
        use_dora=True,
        init_lora_weights="gaussian"
    )

    # Apply PEFT model adaptation
    peft_model = get_peft_model(model, peft_config)

    # Print trainable parameters
    peft_model.print_trainable_parameters()

    # Configure training arguments using SFTConfig
    training_args = SFTConfig(
        output_dir=os.path.join(DATA_DIR, "../results"),
        hub_model_id="lukesutor/SmolVLM-500M-ActivityTracking",
        num_train_epochs=1,
        per_device_train_batch_size=16,
        gradient_accumulation_steps=2,
        warmup_steps=50,
        learning_rate=1e-4,
        weight_decay=0.01,
        logging_steps=25,
        save_strategy="steps",
        save_steps=25,
        save_total_limit=1,
        optim="adamw_torch_fused",
        bf16=True,
        push_to_hub=True,
        report_to="tensorboard",
        remove_unused_columns=False,
        gradient_checkpointing=True,
        dataset_text_field="",
        dataset_kwargs={"skip_prepare_dataset": True},
    )

    image_token_id = processor.tokenizer.additional_special_tokens_ids[
                processor.tokenizer.additional_special_tokens.index("<image>")]

    def collate_fn(examples):
        texts = [processor.apply_chat_template(example, tokenize=False) for example in examples]

        image_inputs = []
        for example in examples:
            # Load image from disk here
            image_filename = example[0]['content'][0]['image_filename']
            image_path = os.path.join(DATA_DIR, image_filename)
            image = Image.open(image_path).convert('RGB')
            image_inputs.append([image])

        batch = processor(text=texts, images=image_inputs, return_tensors="pt", padding=True)
        labels = batch["input_ids"].clone()
        labels[labels == processor.tokenizer.pad_token_id] = -100  # Mask padding tokens in labels
        labels[labels == image_token_id] = -100  # Mask image token IDs in labels

        batch["labels"] = labels

        return batch
    
    trainer = SFTTrainer(
        model=model,
        args=training_args,
        train_dataset=train_dataset,
        eval_dataset=eval_dataset,
        data_collator=collate_fn,
        peft_config=peft_config,
        tokenizer=processor.tokenizer,
    )

    trainer.train()
    trainer.save_model(training_args.output_dir)
    # Save to HF
    trainer.push_to_hub(token=os.environ.get("HF_TOKEN"))


if __name__ == "__main__":
    main()