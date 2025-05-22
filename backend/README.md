# Getting started
1. Create virtual environment ```python -m venv venv```.
2. Activate the environment ```./venv/Scripts/activate```.
3. Install dependencies ```pip install -r requirements.txt```.
4. Use the ```update.sh``` script in the parent directory to create the sidecar and copy it into the Tauri application.

# To compile llama.cpp:
Run these two commands in the ```backend/llama.cpp``` directory:
- ```cmake -B build -DLLAMA_CURL=OFF```
- ```cmake --build build --config Release```