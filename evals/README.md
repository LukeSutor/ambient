# Config-Driven Evaluation System for Local Computer Use

A comprehensive, configuration-driven evaluation framework for ambient project, focusing on task detection and screen understanding capabilities.

## 🎯 Overview

This evaluation system provides:

- **Config-Driven Operation**: All settings controlled via `config.yaml` - no command line arguments needed
- **JSON Schema Integration**: Structured LLM responses with validation and Rust export
- **Automated Task Detection Evaluation**: Measure how well the system identifies actionable tasks from screen states
- **LLM-Powered Analysis**: Uses the same llama.cpp server for consistent evaluation
- **Comprehensive Metrics**: Multi-dimensional scoring across relevance, specificity, completeness, and accuracy
- **Visualization Tools**: Rich charts and graphs for result analysis
- **Easy Extension**: Modular design for adding new evaluation types

## 🚀 Quick Start

### 1. Setup Conda Environment
```bash
# Activate the evals environment
conda activate evals

# Verify packages are installed
conda list | grep -E "(requests|pyyaml|matplotlib|seaborn|pandas|numpy|psutil|tqdm|scikit-learn)"
```

### 2. Configure Your Evaluation
Edit `config.yaml` to customize your evaluation settings:

```yaml
# Key settings you might want to adjust:
evaluation:
  task_detection:
    enabled: true
    data_dir: "./task-detection/data"
    generate_ground_truth: false
    batch_size: 10
  
  visualization:
    enabled: true
    auto_generate: true
  
  log_level: "INFO"
```

### 3. Capture Evaluation Data
Use the dev interface in your Tauri app to capture evaluation data:
1. Navigate to the dev page in your app
2. Perform some actions to get interesting screen states
3. Click "Capture Eval Data" to save the current state
4. Repeat to build up test data

### 4. Run Evaluation
```bash
# Navigate to evals directory and run
cd evals
python eval.py
```

That's it! The system will:
- Read all configuration from `config.yaml`
- Auto-start the LLM server if needed
- Run evaluations on your captured data
- Generate visualizations automatically
- Export schemas for Rust integration

## 📁 Project Structure

```
evals/
├── eval.py                    # Config-driven evaluation runner
├── config.yaml               # All configuration settings
├── requirements.txt           # Python dependencies (conda managed)
├── prompts/                   # Centralized prompts directory
│   └── task-detection.yaml   # Single task detection prompt
├── schemas/                   # Centralized schemas directory
│   └── task_detection.detect_tasks.json  # Task detection schema
├── common/                    # Shared utilities
│   ├── llm_client.py         # LLM client with schema support
│   ├── prompt_manager.py     # YAML-based prompt system
│   ├── data_loader.py        # Data loading and management
│   └── schema_manager.py     # JSON schema handling
└── task-detection/           # Task detection evaluation
    ├── evaluate.py          # Core task detection logic
    ├── visualize.py         # Visualization generation
    ├── data/               # Evaluation data (created by app)
    └── results/            # Evaluation results
```

## ⚙️ Configuration Reference

### Server Settings
```yaml
server:
  executable: "../backend/models/llama-mtmd-cli.exe"
  host: "localhost"
  port: 8080
  auto_start: true
  auto_stop: true
```

### Model Configuration
```yaml
model:
  path: "../backend/models/smol.gguf"
  context_size: 8192
  gpu_layers: 0
```

### Evaluation Settings
```yaml
evaluation:
  task_detection:
    enabled: true                    # Enable/disable this evaluation type
    data_dir: "./task-detection/data" # Where to find evaluation data
    output_file: "./results/task_detection_results.json"
    generate_ground_truth: false     # Whether to generate GT with LLM
    batch_size: 10                   # Processing batch size
    
  visualization:
    enabled: true                    # Generate visualizations
    output_dir: "./plots"            # Where to save charts
    auto_generate: true              # Auto-generate after evaluation
    
  log_level: "INFO"                  # Logging verbosity
```

### JSON Schema Integration
```yaml
schemas:
  schema_dir: "./schemas"            # Centralized schema directory
```

All schemas are now stored as individual JSON files in the `/evals/schemas/` directory with clear naming conventions:
- `task_detection.detect_tasks.json` - Schema for task detection responses
- `task_detection.evaluate_detection.json` - Schema for evaluation results  
- `task_detection.generate_ground_truth.json` - Schema for ground truth generation
- `task_detection.compare_tasks.json` - Schema for task comparison
```

## 🔧 JSON Schema Features

### Structured LLM Responses
The system now enforces JSON schemas for LLM responses, ensuring:
- **Consistent format**: All responses follow defined structure
- **Type validation**: Proper data types for scores, arrays, etc.
- **Required fields**: Critical information is never missing
- **Rust integration**: Schemas automatically exported for main app

### Schema Management
```python
from common import SchemaManager

# Load schemas from config
schema_manager = SchemaManager()

# Get schema for specific evaluation
schema = schema_manager.get_schema_for_prompt('task_detection', 'evaluate_detection')

# Use with LLM client for structured responses
result = llm_client.generate_structured(prompt, schema)
```

### Rust Export
Schemas are automatically exported in Rust format for integration with your main Tauri app:

```rust
// Auto-generated in eval_schemas_export.rs
pub fn get_eval_schema(key: &str) -> Option<&'static str> {
    EVAL_SCHEMAS.get(key).copied()
}
```

## 📊 Evaluation Metrics

The system evaluates task detection across four key dimensions:

### Relevance (0-10)
Are the detected tasks relevant to the current screen state and context?

### Specificity (0-10) 
Are the tasks specific enough to be actionable by a user or automation system?

### Completeness (0-10)
Does the detection cover the main interactive elements and opportunities?

### Accuracy (0-10)
How accurate are the detections compared to ground truth? (when available)

### Overall Score
Balanced composite score across all metrics.

## 🎨 Extending the System

### Adding New Evaluation Types

1. **Create prompt directory and files**:
   ```bash
   mkdir /evals/prompts/my-new-eval
   # Create your prompt YAML files in this directory
   ```

2. **Create schema files**:
   ```bash
   # Create individual schema files for each prompt type
   touch /evals/schemas/my_new_eval.my_prompt.json
   ```

3. **Add configuration section**:
   ```yaml
   evaluation:
     my_new_eval:
       enabled: true
       data_dir: "./my-eval/data"
       custom_setting: "value"
   ```

4. **Implement evaluator**:
   ```python
   # evals/my-eval/evaluate.py
   class MyEvaluator:
       def __init__(self, llm_client, prompt_manager, schema_manager):
           # Your evaluator logic
           self.prompt_manager.get_prompt('my-new-eval', 'my_prompt')
           self.schema_manager.get_schema('my_new_eval.my_prompt')
   ```

5. **Add to main runner**:
   ```python
   # In eval.py main()
   if evaluation_config.get('my_new_eval', {}).get('enabled', False):
       result = run_my_new_eval_evaluation(config)
   ```

## 🐛 Troubleshooting

### Common Issues

1. **Config file not found**
   - Ensure `config.yaml` exists in the `evals` directory
   - Check file permissions and syntax

2. **Server won't start**
   - Verify `server.executable` path in config
   - Check if port is already in use
   - Ensure model file exists at specified path

3. **Schema validation errors**
   - Check schema definitions in config
   - Verify LLM responses match expected format
   - Use `--log-level DEBUG` for detailed error info

4. **Import errors**
   - Ensure conda environment is activated: `conda activate evals`
   - Verify all packages installed correctly

### Performance Tips

- **Adjust batch size**: Reduce `batch_size` for memory-constrained systems
- **Enable caching**: Results are automatically cached to avoid re-evaluation
- **Server management**: Use `auto_start: true` for consistent performance
- **Parallel processing**: Configure in `config.yaml` for faster evaluation

## 🔗 Integration Benefits

### Simplified Usage
- **No command line arguments**: Everything configured in one place
- **Consistent environments**: Conda ensures reproducible setup
- **Automatic workflows**: Evaluation → Visualization → Export all automatic

### Development Workflow
- **Rapid iteration**: Change config and re-run
- **Schema validation**: Catch format errors early
- **Rust integration**: Schemas automatically available in main app
- **Continuous evaluation**: Easy to integrate into development cycle

### Production Ready
- **Robust error handling**: Graceful degradation on failures
- **Comprehensive logging**: Detailed operation tracking
- **Extensible design**: Easy to add new evaluation types
- **Performance optimized**: Efficient processing for large datasets

This config-driven approach makes the evaluation system much easier to use and maintain while providing powerful features for structured LLM interactions and seamless integration with your Rust codebase.
