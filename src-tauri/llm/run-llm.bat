@echo off
set MODEL_PATH=C:\Users\Luke\Desktop\coding\local-computer-use\src-tauri\llm\models\qwen2-vl\Qwen_Qwen2-VL-2B-Instruct-Q4_K_M.gguf
set MMPROJ_PATH=C:\Users\Luke\Desktop\coding\local-computer-use\src-tauri\llm\models\qwen2-vl\qwen2vl-vision-2b.gguf
set IMAGE_PATH=C:\Users\Luke\Desktop\coding\local-computer-use\src-tauri\icons\Square310x310Logo.png
set PROMPT="what is this image? Respond in JSON format with the value \"answer\""
set SCHEMA="{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"title\":\"LLM Answer Schema\",\"description\":\"Schema for constraining LLM output to a single answer string\",\"type\":\"object\",\"properties\":{\"answer\":{\"type\":\"string\",\"description\":\"The LLM's response\"}},\"required\":[\"answer\"],\"additionalProperties\":false}"

C:\Users\Luke\Desktop\coding\local-computer-use\src-tauri\llm\llama.cpp\build\bin\Release\llama-qwen2vl-cli -m %MODEL_PATH% --mmproj %MMPROJ_PATH% --image %IMAGE_PATH% -p %PROMPT% -j %SCHEMA%