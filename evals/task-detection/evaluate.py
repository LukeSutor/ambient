"""
Task detection evaluation implementation.
"""
import json
import logging
import sys
import os
from typing import List, Dict, Any, Optional
from dataclasses import dataclass

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
from data_loader import EvalDataPoint
from schema_manager import SchemaManager
from task_detection_data_loader import TaskDetectionDataLoader, TaskDetectionDataPoint

logger = logging.getLogger(__name__)

@dataclass
class TaskDetectionResult:
    """Result of task detection."""
    data_point_id: str
    analysis: str
    completed_steps: List[int]
    raw_response: str
    tokens_per_second: float

class TaskDetectionEvaluator:
    """Evaluates task detection performance."""
    
    def __init__(self, llm_client: LLMClient, prompt_manager: PromptManager, schema_manager: SchemaManager, data_loader: Optional[TaskDetectionDataLoader] = None):
        """Initialize evaluator with LLM client, prompt manager, schema manager, and optional data loader."""
        self.llm_client = llm_client
        self.prompt_manager = prompt_manager
        self.schema_manager = schema_manager
        self.data_loader = data_loader or TaskDetectionDataLoader()
    
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
            
            return TaskDetectionResult(
                data_point_id=data_point.id,
                analysis=parsed_response.get('analysis', 'No analysis provided'),
                completed_steps=parsed_response.get('completed', []),
                raw_response=response.content,
                tokens_per_second=response.tokens_second
            )
            
        except Exception as e:
            logger.error(f"Task detection failed for data point {data_point.id}: {e}")
            return TaskDetectionResult(
                data_point_id=data_point.id,
                analysis=f"Error: {str(e)}",
                completed_steps=[],
                raw_response="",
                tokens_per_second=0.0
            )
    
    def evaluate_batch(self, data_points: List[TaskDetectionDataPoint]) -> List[TaskDetectionResult]:
        """Evaluate a batch of data points."""
        results = []
        
        for data_point in data_points:
            logger.info(f"Processing data point: {data_point.id}")
            result = self.detect_tasks(data_point)
            results.append(result)
        
        return results
    
    def compute_aggregate_metrics(self, results: List[TaskDetectionResult]) -> Dict[str, Any]:
        """Compute aggregate metrics for evaluation results."""
        if not results:
            return {}
        
        total_data_points = len(results)
        successful_detections = len([r for r in results if r.completed_steps])
        
        return {
            'total_data_points': total_data_points,
            'successful_detections': successful_detections,
            'success_rate': successful_detections / total_data_points if total_data_points > 0 else 0,
            'avg_completed_steps': sum(len(r.completed_steps) for r in results) / total_data_points if total_data_points > 0 else 0
        }
