#!/usr/bin/env python3
"""
Main evaluation script for task detection.

Usage:
    python run_eval.py --data-dir ./data --output ./results/eval_results.json
    python run_eval.py --single-file ./data/eval_123.json --generate-gt
    python run_eval.py --config custom_config.yaml --batch-size 5
"""

import argparse
import json
import logging
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any

# Add parent directory to path for imports
sys.path.append(str(Path(__file__).parent.parent))

from common import LLMClient, PromptManager, DataLoader, EvalDataPoint
from evaluate import TaskDetectionEvaluator, TaskDetectionResult

def setup_logging(log_level: str = "INFO"):
    """Setup logging configuration."""
    logging.basicConfig(
        level=getattr(logging, log_level.upper()),
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        handlers=[
            logging.StreamHandler(sys.stdout),
            logging.FileHandler('eval.log')
        ]
    )

def save_results(results: List[TaskDetectionResult], output_path: str, 
                aggregate_metrics: Dict[str, Any]):
    """Save evaluation results to JSON file."""
    
    # Convert results to serializable format
    results_data = []
    for result in results:
        result_dict = {
            'data_point_id': result.data_point_id,
            'scores': {
                'relevance': result.relevance_score,
                'specificity': result.specificity_score,
                'completeness': result.completeness_score,
                'accuracy': result.accuracy_score,
                'overall': result.overall_score
            },
            'issues': {
                'missed_tasks': result.missed_tasks,
                'false_positives': result.false_positives
            },
            'feedback': result.feedback,
            'detected_tasks_count': len(result.detected_tasks),
            'ground_truth_tasks_count': len(result.ground_truth_tasks) if result.ground_truth_tasks else None
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
    
    logging.info(f"Results saved to: {output_path}")

def main():
    """Main evaluation function."""
    parser = argparse.ArgumentParser(description='Run task detection evaluation')
    
    # Data options
    parser.add_argument('--data-dir', default='./data', 
                       help='Directory containing eval data files')
    parser.add_argument('--single-file', 
                       help='Evaluate a single data file')
    parser.add_argument('--pattern', default='*.json',
                       help='File pattern to match in data directory')
    
    # Configuration
    parser.add_argument('--config', default='../config.yaml',
                       help='Configuration file path')
    parser.add_argument('--prompts-dir', default='./prompts',
                       help='Directory containing prompt files')
    
    # Processing options
    parser.add_argument('--generate-gt', action='store_true',
                       help='Generate ground truth tasks using LLM')
    parser.add_argument('--batch-size', type=int, default=10,
                       help='Batch size for processing data')
    
    # Output options
    parser.add_argument('--output', default='./results/eval_results.json',
                       help='Output file for results')
    parser.add_argument('--log-level', default='INFO',
                       choices=['DEBUG', 'INFO', 'WARNING', 'ERROR'],
                       help='Logging level')
    
    args = parser.parse_args()
    
    # Setup logging
    setup_logging(args.log_level)
    logger = logging.getLogger(__name__)
    
    logger.info("Starting task detection evaluation")
    logger.info(f"Config: {args.config}")
    logger.info(f"Data source: {args.single_file or args.data_dir}")
    
    try:
        # Initialize components
        logger.info("Initializing LLM client...")
        llm_client = LLMClient(args.config)
        
        logger.info("Initializing prompt manager...")
        prompt_manager = PromptManager(args.prompts_dir)
        
        logger.info("Initializing data loader...")
        data_loader = DataLoader(args.data_dir)
        
        logger.info("Initializing evaluator...")
        evaluator = TaskDetectionEvaluator(llm_client, prompt_manager)
        
        # Load data
        if args.single_file:
            logger.info(f"Loading single file: {args.single_file}")
            data_points = [data_loader.load_json_file(args.single_file)]
        else:
            logger.info(f"Loading all data from: {args.data_dir}")
            data_points = data_loader.load_all_data(args.pattern)
        
        if not data_points:
            logger.error("No data points loaded!")
            return 1
        
        logger.info(f"Loaded {len(data_points)} data points")
        
        # Print data statistics
        stats = data_loader.get_data_stats(data_points)
        logger.info(f"Data stats: {stats}")
        
        # Run evaluation
        with llm_client:  # Use context manager for server management
            logger.info("Starting evaluation...")
            results = evaluator.evaluate_batch(
                data_points, 
                generate_ground_truth=args.generate_gt
            )
        
        # Compute aggregate metrics
        logger.info("Computing aggregate metrics...")
        aggregate_metrics = evaluator.compute_aggregate_metrics(results)
        
        # Log summary
        logger.info(f"Evaluation complete!")
        logger.info(f"Average overall score: {aggregate_metrics.get('avg_overall', 0):.2f}")
        logger.info(f"Average relevance: {aggregate_metrics.get('avg_relevance', 0):.2f}")
        logger.info(f"Average specificity: {aggregate_metrics.get('avg_specificity', 0):.2f}")
        logger.info(f"Average completeness: {aggregate_metrics.get('avg_completeness', 0):.2f}")
        
        # Save results
        save_results(results, args.output, aggregate_metrics)
        
        logger.info("Evaluation finished successfully!")
        return 0
        
    except Exception as e:
        logger.error(f"Evaluation failed: {e}", exc_info=True)
        return 1

if __name__ == '__main__':
    sys.exit(main())
