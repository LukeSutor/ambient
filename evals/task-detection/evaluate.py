"""
Task detection evaluation implementation.
"""
import json
import logging
import sys
import os
from tqdm import tqdm
from typing import List, Dict, Any, Optional
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor, as_completed
import time

# Add parent directory to path to import common modules
parent_dir = os.path.join(os.path.dirname(__file__), '..', 'common')
if parent_dir not in sys.path:
    sys.path.insert(0, parent_dir)

# Add current directory for task detection specific modules
current_dir = os.path.dirname(__file__)
if current_dir not in sys.path:
    sys.path.insert(0, current_dir)

from llm_client import LLMClient
from prompt_manager import PromptManager
from schema_manager import SchemaManager
from task_detection_data_loader import TaskDetectionDataLoader, TaskDetectionDataPoint

logger = logging.getLogger(__name__)

@dataclass
class TaskDetectionResult:
    """Result of task detection."""
    filename: str
    analysis: str
    completed_steps: List[int]
    raw_response: str
    correct: bool
    ground_truth: List[int]
    tokens_generated: int
    response_time: float
    tokens_per_second: float

class TaskDetectionEvaluator:
    """Evaluates task detection performance."""
    
    def __init__(self, llm_client: LLMClient, prompt_manager: PromptManager, schema_manager: SchemaManager, config: dict, data_loader: Optional[TaskDetectionDataLoader] = None):
        """Initialize evaluator with LLM client, prompt manager, schema manager, and optional data loader."""
        self.llm_client = llm_client
        self.prompt_manager = prompt_manager
        self.schema_manager = schema_manager
        self.data_loader = data_loader or TaskDetectionDataLoader()
        self.config = config
    
    def get_parallel_config(self) -> Dict[str, Any]:
        """Get parallel processing configuration from config."""
        parallel_config = self.config.get('parallel', {})
        server_config = self.config.get('server', {}).get('startup_config', {})
        
        return {
            'max_workers': server_config.get('np', parallel_config.get('max_concurrent_requests', 1)),
            'request_timeout': parallel_config.get('request_timeout', 60),
            'batch_size': self.config.get('evaluation', {}).get('task_detection', {}).get('batch_size', 10)
        }
    
    def detect_tasks(self, data_point: TaskDetectionDataPoint) -> TaskDetectionResult:
        """Run task detection on a single data point."""
        
        # Use the data loader to prepare prompt data
        prompt_data = self.data_loader.prepare_prompt_data(data_point)
        
        # Get task detection prompt
        detect_prompt = self.prompt_manager.get_prompt(
            'task-detection', 
            'detect_tasks',
            **prompt_data
        )
        
        # Get the schema for structured response
        schema_key = self.data_loader.get_evaluation_schema_key()
        schema = self.schema_manager.get_schema(schema_key)
        if not schema:
            logger.warning(f"No schema found for {schema_key}. Using default response format.")
            schema = None
        
        # Generate task detection with schema constraint
        try:
            response = self.llm_client.generate(detect_prompt, schema)
            
            try:
                parsed_response = json.loads(response.content)
            except json.JSONDecodeError:
                logger.error(f"Failed to parse task detection response: {response.content}")
                parsed_response = {"analysis": "Failed to parse response", "completed": []}

            # Check correctness
            completed_steps = parsed_response.get('completed', [])
            ground_truth = data_point.ground_truth
            correct = set(completed_steps) == set(ground_truth) and len(completed_steps) == len(ground_truth)
            
            return TaskDetectionResult(
                filename=data_point.filename,
                analysis=parsed_response.get('analysis', 'No analysis provided'),
                completed_steps=parsed_response.get('completed', []),
                raw_response=response.content,
                correct=correct,
                ground_truth=ground_truth,
                tokens_generated=response.tokens_generated,
                response_time=response.time_taken,
                tokens_per_second=response.tokens_second
            )
            
        except Exception as e:
            logger.error(f"Task detection failed for data point {data_point.filename}: {e}")
            return TaskDetectionResult(
                filename=data_point.filename,
                analysis=f"Error: {str(e)}",
                completed_steps=[],
                raw_response="",
                correct=False,
                ground_truth=data_point.ground_truth,
                tokens_generated=0,
                response_time=0.0,
                tokens_per_second=0.0
            )
    
    def evaluate_batch(self, data_points: List[TaskDetectionDataPoint]) -> List[TaskDetectionResult]:
        """Evaluate a batch of data points with parallel processing."""
        results = []
        
        # Get the parallelism configuration
        parallel_config = self.get_parallel_config()
        max_workers = parallel_config['max_workers']
        
        # If only one worker or small batch, use sequential processing
        if max_workers <= 1 or len(data_points) <= max_workers:
            for data_point in tqdm(data_points, desc="Evaluating task detection", unit="data point"):
                result = self.detect_tasks(data_point)
                results.append(result)
            return results
        
        # Use parallel processing for larger batches
        results = [None] * len(data_points)  # Pre-allocate to maintain order
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            # Submit all tasks
            future_to_index = {
                executor.submit(self.detect_tasks, data_point): i 
                for i, data_point in enumerate(data_points)
            }
            
            # Process completed tasks with progress bar
            with tqdm(total=len(data_points), desc="Evaluating task detection", unit="data point") as pbar:
                for future in as_completed(future_to_index):
                    index = future_to_index[future]
                    try:
                        result = future.result(timeout=parallel_config['request_timeout'])
                        results[index] = result
                    except Exception as exc:
                        logger.error(f"Data point {index} generated an exception: {exc}")
                        # Create error result
                        data_point = data_points[index]
                        results[index] = TaskDetectionResult(
                            filename=data_point.filename,
                            analysis=f"Error: {str(exc)}",
                            completed_steps=[],
                            raw_response="",
                            correct=False,
                            ground_truth=data_point.ground_truth,
                            tokens_generated=0,
                            response_time=0.0,
                            tokens_per_second=0.0
                        )
                    finally:
                        pbar.update(1)
        
        return results
    
    def evaluate_batch_chunked(self, data_points: List[TaskDetectionDataPoint], chunk_size: Optional[int] = None) -> List[TaskDetectionResult]:
        """Evaluate data points in smaller chunks to manage memory and server load."""
        if chunk_size is None:
            # Use batch_size from parallel config
            parallel_config = self.get_parallel_config()
            chunk_size = parallel_config['batch_size']
        
        all_results = []
        total_chunks = (len(data_points) + chunk_size - 1) // chunk_size
        
        logger.info(f"Processing {len(data_points)} data points in {total_chunks} chunks of size {chunk_size}")
        
        for i in tqdm(range(0, len(data_points), chunk_size), 
                     desc="Processing chunks", unit="chunk", total=total_chunks):
            chunk = data_points[i:i + chunk_size]
            chunk_results = self.evaluate_batch(chunk)
            all_results.extend(chunk_results)
            
            # Optional: Add a small delay between chunks to prevent overwhelming the server
            if i + chunk_size < len(data_points):
                time.sleep(0.1)
        
        return all_results
    
    def get_performance_metrics(self, results: List[TaskDetectionResult]) -> Dict[str, Any]:
        """Get performance metrics specific to parallel processing."""
        if not results:
            return {}
        
        parallel_config = self.get_parallel_config()
        total_time = sum(r.response_time for r in results)
        sequential_time_estimate = total_time  # If run sequentially
        parallel_time_estimate = max(r.response_time for r in results) if results else 0
        
        return {
            'parallel_config': parallel_config,
            'total_response_time': total_time,
            'estimated_sequential_time': sequential_time_estimate,
            'estimated_parallel_time': parallel_time_estimate,
            'estimated_speedup': sequential_time_estimate / parallel_time_estimate if parallel_time_estimate > 0 else 1,
            'average_concurrent_efficiency': len(results) / parallel_config['max_workers'] if parallel_config['max_workers'] > 0 else 1
        }
    
    def compute_aggregate_metrics(self, results: List[TaskDetectionResult]) -> Dict[str, Any]:
        """Compute aggregate metrics for evaluation results."""
        if not results:
            return {}
        
        total_data_points = len(results)
        successful_detections = sum(1 for r in results if r.correct)
        
        return {
            'total_data_points': total_data_points,
            'successful_detections': successful_detections,
            'success_rate': successful_detections / total_data_points if total_data_points > 0 else 0,
            'averate_tokens_generated': sum(r.tokens_generated for r in results) / total_data_points if total_data_points > 0 else 0,
            'average_response_time': sum(r.response_time for r in results) / total_data_points if total_data_points > 0 else 0,
            'average_tokens_per_second': sum(r.tokens_per_second for r in results) / total_data_points if total_data_points > 0 else 0,
        }
