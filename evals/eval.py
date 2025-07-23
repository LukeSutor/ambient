#!/usr/bin/env python3
"""
Main evaluation runner for the local-computer-use project.

This script provides a unified interface for running different types of evaluations.
"""

import argparse
import logging
import sys
import os
from pathlib import Path

def setup_logging(level: str = "INFO"):
    """Setup logging configuration."""
    logging.basicConfig(
        level=getattr(logging, level.upper()),
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.StreamHandler(sys.stdout),
            logging.FileHandler('eval_main.log')
        ]
    )

def run_task_detection(args):
    """Run task detection evaluation."""
    task_detection_dir = Path(__file__).parent / 'task-detection'
    sys.path.insert(0, str(task_detection_dir))
    
    # Import and run task detection
    try:
        import run_eval
        
        # Convert our args to task detection args
        task_args = [
            '--data-dir', args.data_dir,
            '--output', args.output,
            '--config', args.config,
            '--log-level', args.log_level
        ]
        
        if args.generate_gt:
            task_args.append('--generate-gt')
        
        if args.single_file:
            task_args.extend(['--single-file', args.single_file])
        
        if args.batch_size:
            task_args.extend(['--batch-size', str(args.batch_size)])
        
        # Override sys.argv temporarily
        original_argv = sys.argv
        sys.argv = ['run_eval.py'] + task_args
        
        try:
            return run_eval.main()
        finally:
            sys.argv = original_argv
            
    except ImportError as e:
        logging.error(f"Failed to import task detection module: {e}")
        return 1
    finally:
        # Remove from path
        if str(task_detection_dir) in sys.path:
            sys.path.remove(str(task_detection_dir))

def run_visualization(args):
    """Run visualization generation."""
    if not args.results_file:
        logging.error("Results file is required for visualization")
        return 1
    
    if not os.path.exists(args.results_file):
        logging.error(f"Results file not found: {args.results_file}")
        return 1
    
    try:
        task_detection_dir = Path(__file__).parent / 'task-detection'
        sys.path.insert(0, str(task_detection_dir))
        
        from visualize import EvalVisualizer
        
        visualizer = EvalVisualizer()
        output_dir = args.output or './plots'
        visualizer.generate_all_plots(args.results_file, output_dir)
        
        logging.info(f"Visualizations generated in: {output_dir}")
        return 0
        
    except ImportError as e:
        logging.error(f"Failed to import visualization module: {e}")
        return 1
    except Exception as e:
        logging.error(f"Visualization failed: {e}")
        return 1
    finally:
        # Remove from path
        if str(task_detection_dir) in sys.path:
            sys.path.remove(str(task_detection_dir))

def main():
    """Main function."""
    parser = argparse.ArgumentParser(
        description='Local Computer Use Evaluation Suite',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Run task detection evaluation
  python eval.py task-detection --data-dir ./task-detection/data
  
  # Generate ground truth and evaluate
  python eval.py task-detection --data-dir ./task-detection/data --generate-gt
  
  # Evaluate single file
  python eval.py task-detection --single-file ./task-detection/data/eval_123.json
  
  # Generate visualizations
  python eval.py visualize --results-file ./results/eval_results.json
  
  # Custom configuration
  python eval.py task-detection --config custom_config.yaml --batch-size 5
        """
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Evaluation type')
    
    # Task detection evaluation
    task_parser = subparsers.add_parser('task-detection', 
                                       help='Run task detection evaluation')
    task_parser.add_argument('--data-dir', default='./task-detection/data',
                           help='Directory containing eval data')
    task_parser.add_argument('--single-file', help='Single data file to evaluate')
    task_parser.add_argument('--config', default='./config.yaml',
                           help='Configuration file')
    task_parser.add_argument('--generate-gt', action='store_true',
                           help='Generate ground truth using LLM')
    task_parser.add_argument('--batch-size', type=int, default=10,
                           help='Batch size for processing')
    task_parser.add_argument('--output', default='./results/task_detection_results.json',
                           help='Output file for results')
    
    # Visualization
    viz_parser = subparsers.add_parser('visualize',
                                     help='Generate visualization plots')
    viz_parser.add_argument('--results-file', required=True,
                          help='JSON results file to visualize')
    viz_parser.add_argument('--output', default='./plots',
                          help='Output directory for plots')
    
    # Global options
    parser.add_argument('--log-level', default='INFO',
                       choices=['DEBUG', 'INFO', 'WARNING', 'ERROR'],
                       help='Logging level')
    
    args = parser.parse_args()
    
    # Setup logging
    setup_logging(args.log_level)
    logger = logging.getLogger(__name__)
    
    if not args.command:
        parser.print_help()
        return 1
    
    logger.info(f"Starting {args.command} evaluation")
    
    # Route to appropriate handler
    if args.command == 'task-detection':
        return run_task_detection(args)
    elif args.command == 'visualize':
        return run_visualization(args)
    else:
        logger.error(f"Unknown command: {args.command}")
        return 1

if __name__ == '__main__':
    sys.exit(main())
