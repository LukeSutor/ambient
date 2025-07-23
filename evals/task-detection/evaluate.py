"""
Task detection evaluation implementation.
"""
import json
import logging
import numpy as np
from typing import List, Dict, Any, Tuple, Optional
from dataclasses import dataclass
from ..common import LLMClient, PromptManager, EvalDataPoint

logger = logging.getLogger(__name__)

@dataclass
class TaskDetectionResult:
    """Result of task detection evaluation."""
    data_point_id: str
    relevance_score: float
    specificity_score: float
    completeness_score: float
    accuracy_score: float
    overall_score: float
    missed_tasks: List[str]
    false_positives: List[str]
    feedback: str
    detected_tasks: List[Dict[str, Any]]
    ground_truth_tasks: Optional[List[Dict[str, Any]]] = None

class TaskDetectionEvaluator:
    """Evaluates task detection performance."""
    
    def __init__(self, llm_client: LLMClient, prompt_manager: PromptManager):
        """Initialize evaluator with LLM client and prompt manager."""
        self.llm_client = llm_client
        self.prompt_manager = prompt_manager
    
    def evaluate_data_point(self, data_point: EvalDataPoint, 
                          ground_truth_tasks: Optional[List[Dict[str, Any]]] = None) -> TaskDetectionResult:
        """Evaluate task detection for a single data point."""
        
        # Prepare context for evaluation
        screen_summary = data_point.summary or "No summary available"
        detected_tasks_str = json.dumps(data_point.detected_tasks, indent=2)
        ground_truth_str = json.dumps(ground_truth_tasks, indent=2) if ground_truth_tasks else "Not available"
        
        # Get evaluation prompt
        eval_prompt = self.prompt_manager.get_prompt(
            'task-detection', 
            'evaluate_detection',
            screen_summary=screen_summary,
            detected_tasks=detected_tasks_str,
            ground_truth_tasks=ground_truth_str
        )
        
        # Generate evaluation
        try:
            response = self.llm_client.generate(eval_prompt)
            
            # Parse JSON response
            eval_result = json.loads(response)
            
            return TaskDetectionResult(
                data_point_id=data_point.id,
                relevance_score=eval_result.get('relevance', {}).get('score', 0),
                specificity_score=eval_result.get('specificity', {}).get('score', 0),
                completeness_score=eval_result.get('completeness', {}).get('score', 0),
                accuracy_score=eval_result.get('accuracy', {}).get('score', 0),
                overall_score=eval_result.get('overall_score', 0),
                missed_tasks=eval_result.get('missed_tasks', []),
                false_positives=eval_result.get('false_positives', []),
                feedback=eval_result.get('feedback', ''),
                detected_tasks=data_point.detected_tasks,
                ground_truth_tasks=ground_truth_tasks
            )
            
        except (json.JSONDecodeError, KeyError) as e:
            logger.error(f"Failed to parse evaluation response: {e}")
            # Return default result on parse failure
            return TaskDetectionResult(
                data_point_id=data_point.id,
                relevance_score=0.0,
                specificity_score=0.0,
                completeness_score=0.0,
                accuracy_score=0.0,
                overall_score=0.0,
                missed_tasks=[],
                false_positives=[],
                feedback=f"Evaluation failed: {str(e)}",
                detected_tasks=data_point.detected_tasks,
                ground_truth_tasks=ground_truth_tasks
            )
    
    def generate_ground_truth(self, data_point: EvalDataPoint) -> List[Dict[str, Any]]:
        """Generate ground truth tasks for a data point."""
        
        screen_summary = data_point.summary or "No summary available"
        current_state_str = json.dumps(data_point.current_state, indent=2)
        
        # Get ground truth generation prompt
        gt_prompt = self.prompt_manager.get_prompt(
            'task-detection',
            'generate_ground_truth',
            screen_summary=screen_summary,
            current_state=current_state_str
        )
        
        try:
            response = self.llm_client.generate(gt_prompt)
            ground_truth_tasks = json.loads(response)
            
            # Validate structure
            if isinstance(ground_truth_tasks, list):
                return ground_truth_tasks
            else:
                logger.error("Ground truth response is not a list")
                return []
                
        except (json.JSONDecodeError, Exception) as e:
            logger.error(f"Failed to generate ground truth: {e}")
            return []
    
    def calculate_task_similarity(self, task1_desc: str, task2_desc: str) -> float:
        """Calculate similarity between two task descriptions."""
        
        compare_prompt = self.prompt_manager.get_prompt(
            'task-detection',
            'compare_tasks',
            task1_description=task1_desc,
            task2_description=task2_desc
        )
        
        try:
            response = self.llm_client.generate(compare_prompt)
            result = json.loads(response)
            return result.get('similarity_score', 0.0)
        except:
            # Fallback to simple string similarity
            return self._simple_similarity(task1_desc, task2_desc)
    
    def _simple_similarity(self, text1: str, text2: str) -> float:
        """Simple similarity calculation as fallback."""
        words1 = set(text1.lower().split())
        words2 = set(text2.lower().split())
        
        if not words1 and not words2:
            return 1.0
        if not words1 or not words2:
            return 0.0
        
        intersection = len(words1.intersection(words2))
        union = len(words1.union(words2))
        
        return intersection / union if union > 0 else 0.0
    
    def evaluate_batch(self, data_points: List[EvalDataPoint], 
                      generate_ground_truth: bool = False) -> List[TaskDetectionResult]:
        """Evaluate multiple data points."""
        results = []
        
        for i, data_point in enumerate(data_points):
            logger.info(f"Evaluating data point {i+1}/{len(data_points)}: {data_point.id}")
            
            ground_truth = None
            if generate_ground_truth:
                ground_truth = self.generate_ground_truth(data_point)
            
            result = self.evaluate_data_point(data_point, ground_truth)
            results.append(result)
        
        return results
    
    def compute_aggregate_metrics(self, results: List[TaskDetectionResult]) -> Dict[str, Any]:
        """Compute aggregate metrics across all results."""
        if not results:
            return {}
        
        metrics = {
            'avg_relevance': np.mean([r.relevance_score for r in results]),
            'avg_specificity': np.mean([r.specificity_score for r in results]),
            'avg_completeness': np.mean([r.completeness_score for r in results]),
            'avg_accuracy': np.mean([r.accuracy_score for r in results]),
            'avg_overall': np.mean([r.overall_score for r in results]),
            'total_evaluated': len(results),
            'total_missed_tasks': sum(len(r.missed_tasks) for r in results),
            'total_false_positives': sum(len(r.false_positives) for r in results),
            'score_distribution': {
                'relevance': [r.relevance_score for r in results],
                'specificity': [r.specificity_score for r in results],
                'completeness': [r.completeness_score for r in results],
                'accuracy': [r.accuracy_score for r in results],
                'overall': [r.overall_score for r in results]
            }
        }
        
        # Add standard deviations
        for metric in ['relevance', 'specificity', 'completeness', 'accuracy', 'overall']:
            scores = metrics['score_distribution'][metric]
            metrics[f'std_{metric}'] = np.std(scores)
        
        return metrics
