# Updates the python backend inside the Tauri app.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/backend"

# Activate the venv
echo "Activating virtual environment"
./venv/Scripts/activate.bat

# Build the backend library
echo "Building the backend library"
pyinstaller --onefile --paths ./venv/Lib/site-packages ./src/main.py

# Copy the binary file depending on operating system
if [[ "$OSTYPE" == "linux"* ]]; then
    echo "Detected Linux operating system"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Detected Mac operating system"
    # Copy all of mac's necessary files here
elif [[ "$OSTYPE" == "msys" ]]; then
    echo "Detected Windows operating system"
    # Copy backend executable
    cp ./dist/main.exe ../app/src-tauri/binaries/main-x86_64-pc-windows-msvc.exe
else
    echo "Operating System: Unknown"
fi

echo "Backend update process completed"