import argparse
from transformers import AutoModelForImageTextToText, AutoTokenizer
from peft import PeftModel

def merge_lora(base_model_path, lora_path, output_path):
    # Load base model and tokenizer
    model = AutoModelForImageTextToText.from_pretrained(base_model_path)
    tokenizer = AutoTokenizer.from_pretrained(base_model_path)
    # Load LoRA adapter
    model = PeftModel.from_pretrained(model, lora_path)
    # Merge LoRA weights into base model
    model = model.merge_and_unload()
    # Save merged model and tokenizer
    model.save_pretrained(output_path)
    tokenizer.save_pretrained(output_path)
    print(f"Merged model saved to {output_path}")

def main():
    parser = argparse.ArgumentParser(description="Merge a LoRA adapter into a Transformers model.")
    parser.add_argument("--base_model", type=str, required=True, help="Path to the base model")
    parser.add_argument("--lora_path", type=str, required=True, help="Path to the LoRA adapter")
    parser.add_argument("--output_path", type=str, required=True, help="Path to save the merged model")
    args = parser.parse_args()
    merge_lora(args.base_model, args.lora_path, args.output_path)

if __name__ == "__main__":
    main()
