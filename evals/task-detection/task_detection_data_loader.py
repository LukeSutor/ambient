"""
Task detection specific data loader.
"""
import json
import sys
import os
from typing import List, Dict, Any, Type
from dataclasses import dataclass
import logging

# Add parent directory to path to import common modules
parent_dir = os.path.join(os.path.dirname(__file__), '..', 'common')
if parent_dir not in sys.path:
    sys.path.insert(0, parent_dir)

from data_loader import BaseDataLoader, EvalDataPoint, BaseEvalDataPoint

logger = logging.getLogger(__name__)


@dataclass
class TaskDetectionDataPoint(EvalDataPoint):
    """Task detection specific data point with additional validation and methods."""
    
    def validate(self) -> bool:
        """Enhanced validation for task detection data points."""
        # Call parent validation first
        if not super().validate():
            return False
        
        # Task detection specific validation
        if not self.current_state or 'data' not in self.current_state:
            return False
        
        # Ensure data is a list
        if not isinstance(self.current_state['data'], list):
            return False
        
        # Check for minimum viable screen content
        if not self._has_meaningful_content():
            return False
        
        return True
    
    def _has_meaningful_content(self) -> bool:
        """Check if the data point has meaningful screen content."""
        if not self.current_state or 'data' not in self.current_state:
            return False
        
        total_text_length = 0
        for item in self.current_state['data']:
            if 'text_content' in item and isinstance(item['text_content'], list):
                for text in item['text_content']:
                    if isinstance(text, str):
                        total_text_length += len(text.strip())
        
        # Require at least 10 characters of meaningful text
        return total_text_length >= 10
    
    def get_screen_applications(self) -> List[str]:
        """Get list of applications present in the current state."""
        apps = []
        if self.current_state and 'data' in self.current_state:
            for item in self.current_state['data']:
                app_name = item.get('application_name')
                if app_name and app_name not in apps:
                    apps.append(app_name)
        return apps
    
    def get_total_screen_text_length(self) -> int:
        """Get total length of screen text."""
        total_length = 0
        if self.current_state and 'data' in self.current_state:
            for item in self.current_state['data']:
                if 'text_content' in item and isinstance(item['text_content'], list):
                    for text in item['text_content']:
                        if isinstance(text, str):
                            total_length += len(text.strip())
        return total_length


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
        # The data point's validate() method includes our filtering logic
        return data_point.validate() and data_point.get_total_screen_text_length() >= self.min_screen_text_length
    
    def prepare_prompt_data(self, data_point: TaskDetectionDataPoint) -> Dict[str, Any]:
        """
        Prepare data point for task detection prompt generation.
        
        Returns:
            Dictionary with keys: previous_summary, text, active_url, tasks
        """
        return {
            'previous_summary': data_point.summary or "No previous summary available",
            'text': self._extract_screen_text(data_point),
            'active_url': self._extract_active_url(data_point),
            'tasks': json.dumps(data_point.detected_tasks, indent=2) if data_point.detected_tasks else "[]"
        }
    
    def get_evaluation_schema_key(self) -> str:
        """Get the schema key for task detection evaluation."""
        return "task_detection.detect_tasks"
    
    def _extract_screen_text(self, data_point: TaskDetectionDataPoint) -> str:
        """
        Extract screen text from current state.
        
        Combines all text content from all applications in the current state.
        """
        if not data_point.current_state or 'data' not in data_point.current_state:
            return "No screen text available"
        
        text_parts = []
        for item in data_point.current_state['data']:
            if 'text_content' in item and isinstance(item['text_content'], list):
                # Filter out empty strings and very short text snippets
                meaningful_text = [
                    text.strip() for text in item['text_content'] 
                    if isinstance(text, str) and len(text.strip()) > 2
                ]
                text_parts.extend(meaningful_text)
        
        return "\n".join(text_parts) if text_parts else "No screen text available"
    
    def _extract_active_url(self, data_point: TaskDetectionDataPoint) -> str:
        """
        Extract active URL from current state.
        
        Looks for browser application data and extracts URL/title information.
        """
        if not data_point.current_state or 'data' not in data_point.current_state:
            return "No URL available"
        
        # Look for browser applications
        browser_apps = ['Brave Browser', 'Chrome', 'Firefox', 'Safari', 'Edge']
        
        for item in data_point.current_state['data']:
            app_name = item.get('application_name', '')
            
            # Check if this is a browser application
            if any(browser in app_name for browser in browser_apps):
                if 'text_content' in item and item['text_content']:
                    # First text content in browser is often the URL/title
                    return item['text_content'][0]
        
        return "No URL available"
    
    def filter_for_task_detection(self, data_points: List[TaskDetectionDataPoint]) -> List[TaskDetectionDataPoint]:
        """
        Apply task detection specific filtering to data points.
        
        This is a convenience method that applies standard task detection filters.
        """
        filtered = []
        
        for point in data_points:
            # Skip if too many tasks (likely corrupted data)
            if len(point.detected_tasks) > 50:
                continue
            
            # Skip if current state is too complex (might be corrupted)
            if (point.current_state and 'data' in point.current_state and 
                len(point.current_state['data']) > 100):
                continue
            
            filtered.append(point)
        
        logger.info(f"Task detection filtering: {len(data_points)} -> {len(filtered)} data points")
        return filtered
    
    def get_task_detection_stats(self, data_points: List[TaskDetectionDataPoint]) -> Dict[str, Any]:
        """
        Get task detection specific statistics.
        
        Extends base statistics with task detection specific metrics.
        """
        stats = self.get_data_stats(data_points)
        
        if not data_points:
            return stats
        
        # Add task detection specific stats
        screen_text_lengths = [point.get_total_screen_text_length() for point in data_points]
        url_availability = [self._extract_active_url(point) != "No URL available" for point in data_points]
        summary_availability = [bool(point.summary) for point in data_points]
        task_counts = [len(point.detected_tasks) for point in data_points]
        
        stats.update({
            'avg_screen_text_length': sum(screen_text_lengths) / len(screen_text_lengths),
            'min_screen_text_length': min(screen_text_lengths),
            'max_screen_text_length': max(screen_text_lengths),
            'url_availability_rate': sum(url_availability) / len(url_availability),
            'summary_availability_rate': sum(summary_availability) / len(summary_availability),
            'data_points_with_no_tasks': len([p for p in data_points if len(p.detected_tasks) == 0]),
            'application_types': self._get_application_types(data_points),
            'task_counts': task_counts,
            'avg_tasks': sum(task_counts) / len(task_counts),
            'min_tasks': min(task_counts),
            'max_tasks': max(task_counts)
        })
        
        return stats
    
    def _get_application_types(self, data_points: List[TaskDetectionDataPoint]) -> Dict[str, int]:
        """Get count of different application types in the data."""
        app_counts = {}
        
        for point in data_points:
            apps = point.get_screen_applications()
            for app_name in apps:
                app_counts[app_name] = app_counts.get(app_name, 0) + 1
        
        return app_counts
