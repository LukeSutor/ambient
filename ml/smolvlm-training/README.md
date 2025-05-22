Installing packages on HiPerGator:
1. load conda ```module load conda```
2. Activate env ```mamba activate vlm```
3. Load cuda ```module load cuda/12.4.1``` (for flashattention building)
4. Set cuda home env variable ```export CUDA_HOME=$HPC_CUDA_DIR```
5. Load GCC ```module load gcc/12.2.0``` (for flashattention building)
6. Install packages ```pip install -r requirements.txt```
7. Install flash attention ```MAX_JOBS=4 pip install flash-attn --no-build-isolation```