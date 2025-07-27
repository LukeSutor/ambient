#!/usr/bin/env python3
import logging
import sys
import os
import yaml
from pathlib import Path
from datetime import datetime

def setup_logging(level: str = "INFO"):
    """Setup logging configuration."""    
    logging.basicConfig(
        level=getattr(logging, level.upper()),
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.StreamHandler(sys.stdout)
        ]
    )

def load_config(config_path: str = "config.yaml") -> dict:
    """Load configuration from YAML file."""
    # Make config path relative to this script's directory
    script_dir = Path(__file__).parent
    if not os.path.isabs(config_path):
        config_path = script_dir / config_path
    
    if not os.path.exists(config_path):
        raise FileNotFoundError(f"Configuration file not found: {config_path}")
    
    with open(config_path, 'r') as f:
        return yaml.safe_load(f)

def run_task_detection_evaluation(config: dict):
    """Run task detection evaluation based on configuration."""
    logger = logging.getLogger(__name__)
    script_dir = Path(__file__).parent
    task_detection_dir = script_dir / 'task-detection'
    common_dir = script_dir / 'common'
    
    # Add paths to Python path
    sys.path.insert(0, str(task_detection_dir))
    sys.path.insert(0, str(common_dir))
    
    try:
        from llm_client import LLMClient
        from prompt_manager import PromptManager
        from schema_manager import SchemaManager
        from evaluate import TaskDetectionEvaluator
        from task_detection_data_loader import TaskDetectionDataLoader
        
        eval_config = config['evaluation']['task_detection']
        
        if not eval_config.get('enabled', True):
            logger.info("Task detection evaluation disabled in config")
            return 0
        
        logger.info("Initializing evaluation components...")
        
        # Initialize components with script-relative paths
        config_path = str(script_dir / 'config.yaml')
        llm_client = LLMClient(config_path)
        prompt_manager = PromptManager(str(script_dir / 'prompts'))
        
        # Make data directory relative to script if not absolute
        data_dir = eval_config['data_dir']
        if not os.path.isabs(data_dir):
            data_dir = str(script_dir / data_dir)
        
        # Use the task detection specific data loader
        task_data_loader = TaskDetectionDataLoader(data_dir)
        schema_manager = SchemaManager(config_path)
        evaluator = TaskDetectionEvaluator(llm_client, prompt_manager, schema_manager, config, task_data_loader)
        
        # Load data
        logger.info(f"Loading data from: {eval_config['data_dir']}")
        data_points = task_data_loader.load_all_data()
        
        if not data_points:
            logger.warning("No data points found!")
            return 0
        
        logger.info(f"Loaded {len(data_points)} data points")
        
        # Print data statistics
        stats = task_data_loader.get_task_detection_stats(data_points)
        logger.info(f"Data stats: {stats}")
        
        # Run evaluation
        with llm_client:  # Use context manager for server management
            logger.info("Starting task detection...")
            results = evaluator.evaluate_batch(data_points)
        
        # Compute aggregate metrics
        logger.info("Computing aggregate metrics...")
        aggregate_metrics = evaluator.compute_aggregate_metrics(results)
        
        # Log summary
        logger.info(f"Task detection complete!")
        logger.info(f"Success rate: {aggregate_metrics.get('success_rate', 0):.2f}")
        logger.info(f"Total data points: {aggregate_metrics.get('total_data_points', 0)}")
        logger.info(f"Successful detections: {aggregate_metrics.get('successful_detections', 0)}")
        logger.info(f"Average tokens generated: {aggregate_metrics.get('averate_tokens_generated', 0):.2f}")
        logger.info(f"Average response time: {aggregate_metrics.get('average_response_time', 0):.2f} seconds")
        logger.info(f"Average tokens per second: {aggregate_metrics.get('average_tokens_per_second', 0):.2f}")
        
        # Save results
        output_file = eval_config['output_file']
        # Make output file relative to script if not absolute
        if not os.path.isabs(output_file):
            output_file = str(script_dir / output_file)
        save_results(results, output_file, aggregate_metrics)
        
        # Generate visualizations if enabled
        viz_config = config['evaluation']['visualization']
        if viz_config.get('enabled', True) and viz_config.get('auto_generate', True):
            logger.info("Generating visualizations...")
            generate_visualizations(output_file, viz_config['output_dir'])
        
        logger.info("Task detection evaluation finished successfully!")
        return 0
        
    except Exception as e:
        logger.error(f"Task detection evaluation failed: {e}", exc_info=True)
        return 1
    finally:
        # Remove from path
        if str(task_detection_dir) in sys.path:
            sys.path.remove(str(task_detection_dir))
        if str(common_dir) in sys.path:
            sys.path.remove(str(common_dir))

def save_results(results, output_path: str, aggregate_metrics: dict):
    """Save evaluation results to JSON file."""
    from datetime import datetime
    import json
    
    logger = logging.getLogger(__name__)
    
    # Convert results to serializable format
    results_data = []
    for result in results:
        result_dict = {
            'correct': result.correct,
            'analysis': result.analysis,
            'completed_steps': result.completed_steps,
            'ground_truth': result.ground_truth,
            'raw_response': result.raw_response
        }
        results_data.append(result_dict)
    
    # Prepare output data
    output_data = {
        'evaluation_info': {
            'timestamp': datetime.now().isoformat(),
            'total_data_points': len(results),
            'evaluation_type': 'task_detection'
        },
        'aggregate_metrics': aggregate_metrics,
        'individual_results': results_data
    }
    
    # Create output directory if needed
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    
    # Save results
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(output_data, f, indent=2, ensure_ascii=False)
    
    logger.info(f"Results saved to: {output_path}")

def generate_visualizations(results_file: str, output_dir: str):
    """Generate visualization plots."""
    logger = logging.getLogger(__name__)
    script_dir = Path(__file__).parent
    task_detection_dir = script_dir / 'task-detection'
    sys.path.insert(0, str(task_detection_dir))
    
    try:
        from visualize import EvalVisualizer
        
        # Make output dir relative to script if not absolute
        if not os.path.isabs(output_dir):
            output_dir = str(script_dir / output_dir)
        
        visualizer = EvalVisualizer()
        visualizer.generate_all_plots(results_file, output_dir)
        
        logger.info(f"Visualizations generated in: {output_dir}")
        
    except ImportError as e:
        logger.warning(f"Visualization module not available: {e}")
    except Exception as e:
        logger.error(f"Visualization failed: {e}")
    finally:
        # Remove from path
        if str(task_detection_dir) in sys.path:
            sys.path.remove(str(task_detection_dir))

def main():
    """Main function."""
    logger = logging.getLogger(__name__)
    
    try:
        # Load configuration
        config = load_config()
        
        # Setup logging
        log_level = config.get('evaluation', {}).get('log_level', 'INFO')
        setup_logging(log_level)
        
        logger.info("Starting evaluation system")
        logger.info(f"Configuration loaded from: config.yaml")
        
        # Create results directory relative to script
        script_dir = Path(__file__).parent
        results_dir = config.get('evaluation', {}).get('results_dir', './results')
        if not os.path.isabs(results_dir):
            results_dir = script_dir / results_dir
        os.makedirs(results_dir, exist_ok=True)
        
        # Run evaluations based on configuration
        evaluation_config = config.get('evaluation', {})
        
        exit_code = 0
        
        # Task detection evaluation
        if evaluation_config.get('task_detection', {}).get('enabled', True):
            logger.info("Running task detection evaluation...")
            result = run_task_detection_evaluation(config)
            if result != 0:
                exit_code = result
        
        # Future: Add other evaluation types here
        # if evaluation_config.get('screen_understanding', {}).get('enabled', False):
        #     logger.info("Running screen understanding evaluation...")
        #     result = run_screen_understanding_evaluation(config)
        #     if result != 0:
        #         exit_code = result
        
        if exit_code == 0:
            logger.info("All evaluations completed successfully!")
        else:
            logger.error("Some evaluations failed!")
        
        return exit_code
        
    except FileNotFoundError as e:
        print(f"Error: {e}")
        print("Please ensure config.yaml exists in the current directory.")
        return 1
    except Exception as e:
        if 'logger' in locals():
            logger.error(f"Evaluation system failed: {e}", exc_info=True)
        else:
            print(f"Fatal error: {e}")
        return 1

if __name__ == '__main__':
    sys.exit(main())
