"""
Task detection specific data loader.
"""
import json
import sys
import os
from typing import List, Dict, Any, Optional, Iterator, TypeVar, Generic, Type
from dataclasses import dataclass
import logging

# Add parent directory to path to import common modules
parent_dir = os.path.join(os.path.dirname(__file__), '..', 'common')
if parent_dir not in sys.path:
    sys.path.insert(0, parent_dir)

from data_loader import BaseDataLoader, BaseEvalDataPoint

logger = logging.getLogger(__name__)


@dataclass
class TaskDetectionDataPoint(BaseEvalDataPoint):
    """Task detection specific data point with additional methods."""
    prev_prev_state: Dict[str, Any]
    prev_prev_summary: str
    prev_state: Dict[str, Any]
    screen_diff: Dict[str, Any]
    active_tasks: List[Dict[str, Any]]
    formatted_tasks: List[Dict[str, Any]]
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'TaskDetectionDataPoint':
        """Create TaskDetectionDataPoint from dictionary."""
        try:
            obj = cls(
            filename="",  # Placeholder for filename, will be set later
            timestamp=data['timestamp'],
            ground_truth=data['ground_truth_completed_step_ids'],
            prev_prev_state=data['prev_prev_screen_state'],
            prev_prev_summary=data['prev_prev_summary'],
            prev_state=data['prev_screen_state'],
            screen_diff=data['screen_diff_markdown'],
            active_tasks=data['active_tasks'],
            formatted_tasks=data['formatted_tasks'],
            )
        except KeyError as e:
            logger.error(f"Missing required field in data point: {e}")
            raise ValueError(f"Invalid data point format: {e}")
        return obj
    
    def get_screen_applications(self) -> List[str]:
        """Get list of applications present in the current state."""
        apps = []
        if self.current_state and 'data' in self.current_state:
            for item in self.current_state['data']:
                app_name = item.get('application_name')
                if app_name and app_name not in apps:
                    apps.append(app_name)
        return apps


class TaskDetectionDataLoader(BaseDataLoader[TaskDetectionDataPoint]):
    """Data loader specifically for task detection evaluation."""
    
    def __init__(self, data_dir: str = "data", min_screen_text_length: int = 10):
        """Initialize task detection data loader."""
        super().__init__(data_dir)
        self.min_screen_text_length = min_screen_text_length
    
    def get_data_point_class(self) -> Type[TaskDetectionDataPoint]:
        """Return the TaskDetectionDataPoint class."""
        return TaskDetectionDataPoint
    
    def should_include_data_point(self, data_point: TaskDetectionDataPoint) -> bool:
        """
        Determine if a data point should be included for task detection evaluation.
        
        Uses the enhanced validation in TaskDetectionDataPoint.
        """
        return True
    
    def prepare_prompt_data(self, data_point: TaskDetectionDataPoint) -> Dict[str, Any]:
        """
        Prepare data point for task detection prompt generation.
        
        Returns:
            Dictionary with keys: previous_summary, text, active_url, tasks
        """
        return {
            'previous_summary': data_point.prev_prev_summary or "No previous summary available",
            'text': data_point.screen_diff,
            'active_url': data_point.prev_state.get('active_url', ''),
            'tasks': data_point.formatted_tasks
        }
    
    def get_evaluation_schema_key(self) -> str:
        """Get the schema key for task detection evaluation."""
        return "task_detection.detect_tasks"
    
    def get_task_detection_stats(self, data_points: List[TaskDetectionDataPoint]) -> Dict[str, Any]:
        """
        Get task detection specific statistics.
        
        Extends base statistics with task detection specific metrics.
        """
        stats = self.get_data_stats(data_points)
        
        if not data_points:
            return stats
        
        # Add task detection specific stats
        task_counts = [len(point.ground_truth) for point in data_points]
        
        stats.update({
            'task_counts': task_counts,
            'avg_tasks': sum(task_counts) / len(task_counts),
            'min_tasks': min(task_counts),
            'max_tasks': max(task_counts)
        })
        
        return stats
