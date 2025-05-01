import os
import shutil
import re

# Define the data directory relative to the script location
data_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../data")

# Clean the dataset by removing all the "sub" images and naming based on operating system/task
def main():
    save_dir = os.path.join(data_dir, "images")
    os.makedirs(save_dir, exist_ok=True)

    # Loop through all files for each image directory,
    # copying images to the dataset and deleting unneeded ones.
    data_dirs = ["linux"]#, "windows", "macos", "web"]
    for subdir in data_dirs:
        # Loop through all images in the subdirectory
        i = 0
        base_dir = os.path.join(data_dir, subdir)
        for filename in os.listdir(base_dir):
            current_file_path = os.path.join(base_dir, filename)

            # Ensure we are dealing with files, not directories
            if not os.path.isfile(current_file_path):
                continue

            # Delete sub-images
            if "_sub" in filename:
                try:
                    os.remove(current_file_path) # Use full path
                except OSError as e:
                    print(f"Error deleting file {current_file_path}: {e}")
            else:
                # Construct the new path for the file in save_dir
                new_path = os.path.join(save_dir, f"{subdir}_{i}.png")
                try:
                    shutil.move(current_file_path, new_path) # Use full source path
                    print(f"Moved: {current_file_path} to {new_path}")
                    i += 1
                except OSError as e:
                    print(f"Error moving file {current_file_path} to {new_path}: {e}")
            



if __name__ == "__main__":
    main()