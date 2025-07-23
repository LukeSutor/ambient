"""
Task detection evaluation implementation.
"""
import json
import logging
from typing import List, Dict, Any, Optional
from dataclasses import dataclass
from ..common import LLMClient, PromptManager, EvalDataPoint, SchemaManager

logger = logging.getLogger(__name__)

@dataclass
class TaskDetectionResult:
    """Result of task detection."""
    data_point_id: str
    analysis: str
    completed_steps: List[int]
    raw_response: str

class TaskDetectionEvaluator:
    """Evaluates task detection performance."""
    
    def __init__(self, llm_client: LLMClient, prompt_manager: PromptManager, schema_manager: SchemaManager):
        """Initialize evaluator with LLM client, prompt manager, and schema manager."""
        self.llm_client = llm_client
        self.prompt_manager = prompt_manager
        self.schema_manager = schema_manager
    
    def detect_tasks(self, data_point: EvalDataPoint) -> TaskDetectionResult:
        """Run task detection on a single data point."""
        
        # Prepare context for task detection
        previous_summary = data_point.summary or "No previous summary available"
        text = data_point.screen_text or "No screen text available"
        active_url = data_point.active_url or "No URL available"
        tasks = json.dumps(data_point.detected_tasks, indent=2) if hasattr(data_point, 'detected_tasks') else "[]"
        
        # Get task detection prompt
        detect_prompt = self.prompt_manager.get_prompt(
            'task-detection', 
            'detect_tasks',
            previous_summary=previous_summary,
            text=text,
            active_url=active_url,
            tasks=tasks
        )
        
        # Get the schema for structured response
        schema = self.schema_manager.get_schema('task_detection.detect_tasks')
        
        # Generate task detection with schema constraint
        try:
            if schema:
                response = self.llm_client.generate_structured(detect_prompt, schema)
            else:
                response = self.llm_client.generate(detect_prompt)
            
            # Parse the response
            if isinstance(response, str):
                try:
                    parsed_response = json.loads(response)
                except json.JSONDecodeError:
                    logger.error(f"Failed to parse task detection response: {response}")
                    parsed_response = {"analysis": "Failed to parse response", "completed": []}
            else:
                parsed_response = response
            
            return TaskDetectionResult(
                data_point_id=data_point.id,
                analysis=parsed_response.get('analysis', 'No analysis provided'),
                completed_steps=parsed_response.get('completed', []),
                raw_response=json.dumps(response) if not isinstance(response, str) else response
            )
            
        except Exception as e:
            logger.error(f"Task detection failed for data point {data_point.id}: {e}")
            return TaskDetectionResult(
                data_point_id=data_point.id,
                analysis=f"Error: {str(e)}",
                completed_steps=[],
                raw_response=""
            )
    
    def evaluate_batch(self, data_points: List[EvalDataPoint]) -> List[TaskDetectionResult]:
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
