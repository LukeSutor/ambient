#!/usr/bin/env python3
"""
Example usage of the new task detection data loader.
"""
import sys
import os
from pathlib import Path

# Add the task-detection directory to the path
sys.path.insert(0, str(Path(__file__).parent))
sys.path.insert(0, str(Path(__file__).parent / '..' / 'common'))

from task_detection_data_loader import TaskDetectionDataLoader
from data_loader import EvalDataPoint
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

def main():
    """Demonstrate the new task detection data loader."""
    
    # Initialize the task detection data loader
    data_dir = "./data"  # Adjust path as needed
    loader = TaskDetectionDataLoader(data_dir)
    
    logger.info("Loading data with task detection specific filtering...")
    
    # Load all data (with automatic filtering)
    data_points = loader.load_all_data()
    logger.info(f"Loaded {len(data_points)} data points")
    
    if not data_points:
        logger.warning("No data points found. Create some test data in ./data/ directory")
        return
    
    # Apply additional task detection filtering
    filtered_data = loader.filter_for_task_detection(data_points)
    logger.info(f"After task detection filtering: {len(filtered_data)} data points")
    
    # Get comprehensive statistics
    stats = loader.get_task_detection_stats(filtered_data)
    logger.info("Task detection specific statistics:")
    for key, value in stats.items():
        logger.info(f"  {key}: {value}")
    
    # Demonstrate prompt data preparation
    if filtered_data:
        sample_point = filtered_data[0]
        prompt_data = loader.prepare_prompt_data(sample_point)
        
        logger.info("\nSample prompt data preparation:")
        for key, value in prompt_data.items():
            if len(str(value)) > 100:
                logger.info(f"  {key}: {str(value)[:100]}...")
            else:
                logger.info(f"  {key}: {value}")
        
        # Show schema key
        schema_key = loader.get_evaluation_schema_key()
        logger.info(f"\nSchema key for this evaluation: {schema_key}")

if __name__ == '__main__':
    main()
