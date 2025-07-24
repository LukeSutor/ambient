"""
Data loading utilities for evaluation data.
"""
import json
import os
import glob
from typing import List, Dict, Any, Optional, Iterator, TypeVar, Generic, Type
from dataclasses import dataclass
from abc import ABC, abstractmethod
import logging

logger = logging.getLogger(__name__)

@dataclass
class BaseEvalDataPoint(ABC):
    """Base class for evaluation data points."""
    id: str
    timestamp: str
    metadata: Dict[str, Any]
    
    @classmethod
    @abstractmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'BaseEvalDataPoint':
        """Create data point from dictionary. Must be implemented by subclasses."""
        pass
    
    @abstractmethod
    def to_dict(self) -> Dict[str, Any]:
        """Convert data point to dictionary. Must be implemented by subclasses."""
        pass
    
    @abstractmethod
    def validate(self) -> bool:
        """Validate the data point structure. Must be implemented by subclasses."""
        pass

@dataclass
class EvalDataPoint(BaseEvalDataPoint):
    """Standard evaluation data point for screen-based evaluations."""
    prev_prev_state: Optional[Dict[str, Any]]
    prev_state: Optional[Dict[str, Any]]
    current_state: Dict[str, Any]
    screen_diff: Optional[Dict[str, Any]]
    detected_tasks: List[Dict[str, Any]]
    summary: Optional[str]
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'EvalDataPoint':
        """Create EvalDataPoint from dictionary."""
        return cls(
            id=data.get('id', ''),
            timestamp=data.get('timestamp', ''),
            metadata=data.get('metadata', {}),
            prev_prev_state=data.get('prev_prev_state'),
            prev_state=data.get('prev_state'),
            current_state=data.get('current_state', {}),
            screen_diff=data.get('screen_diff'),
            detected_tasks=data.get('detected_tasks', []),
            summary=data.get('summary')
        )
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert EvalDataPoint to dictionary."""
        return {
            'id': self.id,
            'timestamp': self.timestamp,
            'metadata': self.metadata,
            'prev_prev_state': self.prev_prev_state,
            'prev_state': self.prev_state,
            'current_state': self.current_state,
            'screen_diff': self.screen_diff,
            'detected_tasks': self.detected_tasks,
            'summary': self.summary
        }
    
    def validate(self) -> bool:
        """Validate the EvalDataPoint structure."""
        # Basic validation
        if not self.id or not self.timestamp:
            return False
        
        # Must have current state
        if not isinstance(self.current_state, dict):
            return False
        
        # detected_tasks must be a list
        if not isinstance(self.detected_tasks, list):
            return False
        
        # metadata must be a dict
        if not isinstance(self.metadata, dict):
            return False
        
        return True

# Type variable for generic data point types
DataPointType = TypeVar('DataPointType', bound=BaseEvalDataPoint)

class BaseDataLoader(ABC, Generic[DataPointType]):
    """Base class for loading and managing evaluation data."""
    
    def __init__(self, data_dir: str = "data"):
        """Initialize data loader with data directory."""
        self.data_dir = data_dir
    
    @abstractmethod
    def get_data_point_class(self) -> Type[DataPointType]:
        """Return the data point class this loader uses."""
        pass
        
    def load_json_file(self, filepath: str) -> DataPointType:
        """Load a single JSON eval data file."""
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                data = json.load(f)
            data_point_class = self.get_data_point_class()
            data_point = data_point_class.from_dict(data)
            
            # Validate the data point
            if not data_point.validate():
                logger.warning(f"Data point validation failed for {filepath}")
                raise ValueError(f"Invalid data point structure in {filepath}")
                
            return data_point
        except Exception as e:
            logger.error(f"Failed to load {filepath}: {e}")
            raise
    
    def load_all_data(self, pattern: str = "*.json") -> List[DataPointType]:
        """Load all eval data files matching pattern."""
        data_points = []
        
        if not os.path.exists(self.data_dir):
            logger.warning(f"Data directory not found: {self.data_dir}")
            return data_points
        
        pattern_path = os.path.join(self.data_dir, pattern)
        json_files = glob.glob(pattern_path)
        
        logger.info(f"Found {len(json_files)} data files")
        
        for filepath in sorted(json_files):
            try:
                data_point = self.load_json_file(filepath)
                # Apply evaluation-specific filtering
                if self.should_include_data_point(data_point):
                    data_points.append(data_point)
            except Exception as e:
                logger.error(f"Skipping {filepath}: {e}")
                continue
        
        return data_points
    
    def load_data_batch(self, pattern: str = "*.json", batch_size: int = 10) -> Iterator[List[DataPointType]]:
        """Load data in batches for memory efficiency."""
        if not os.path.exists(self.data_dir):
            logger.warning(f"Data directory not found: {self.data_dir}")
            return
        
        pattern_path = os.path.join(self.data_dir, pattern)
        json_files = sorted(glob.glob(pattern_path))
        
        batch = []
        for filepath in json_files:
            try:
                data_point = self.load_json_file(filepath)
                if self.should_include_data_point(data_point):
                    batch.append(data_point)
                    
                    if len(batch) >= batch_size:
                        yield batch
                        batch = []
            except Exception as e:
                logger.error(f"Skipping {filepath}: {e}")
                continue
        
        # Yield remaining batch
        if batch:
            yield batch
    
    @abstractmethod
    def should_include_data_point(self, data_point: DataPointType) -> bool:
        """Determine if a data point should be included for this evaluation type."""
        pass
    
    @abstractmethod
    def prepare_prompt_data(self, data_point: DataPointType) -> Dict[str, Any]:
        """Prepare data point for prompt generation."""
        pass
    
    @abstractmethod
    def get_evaluation_schema_key(self) -> str:
        """Get the schema key for this evaluation type."""
        pass
    
    def get_data_stats(self, data_points: List[DataPointType]) -> Dict[str, Any]:
        """Get statistics about the loaded data."""
        if not data_points:
            return {}
        
        stats = {
            'total_count': len(data_points),
            'metadata_keys': set()
        }
        
        # Collect all metadata keys
        for point in data_points:
            stats['metadata_keys'].update(point.metadata.keys())
        
        stats['metadata_keys'] = list(stats['metadata_keys'])
        
        return stats
    
    def save_data(self, data_point: DataPointType, filename: Optional[str] = None) -> str:
        """Save a data point to JSON file."""
        if not os.path.exists(self.data_dir):
            os.makedirs(self.data_dir)
        
        if filename is None:
            filename = f"eval_data_{data_point.id}.json"
        
        filepath = os.path.join(self.data_dir, filename)
        
        # Use the data point's to_dict method
        data_dict = data_point.to_dict()
        
        with open(filepath, 'w', encoding='utf-8') as f:
            json.dump(data_dict, f, indent=2, ensure_ascii=False)
        
        logger.info(f"Saved data to: {filepath}")
        return filepath

# Legacy DataLoader class for backward compatibility
class DataLoader(BaseDataLoader[EvalDataPoint]):
    """Legacy data loader for backward compatibility."""
    
    def get_data_point_class(self) -> Type[EvalDataPoint]:
        """Return the EvalDataPoint class."""
        return EvalDataPoint
    
    def should_include_data_point(self, data_point: EvalDataPoint) -> bool:
        """Include all data points by default."""
        return True
    
    def prepare_prompt_data(self, data_point: EvalDataPoint) -> Dict[str, Any]:
        """Basic prompt data preparation."""
        return {
            'summary': data_point.summary or "No previous summary available",
            'screen_text': self._extract_screen_text(data_point),
            'active_url': self._extract_active_url(data_point),
            'tasks': json.dumps(data_point.detected_tasks, indent=2)
        }
    
    def get_evaluation_schema_key(self) -> str:
        """Default schema key."""
        return "default"
    
    def get_data_stats(self, data_points: List[EvalDataPoint]) -> Dict[str, Any]:
        """Get statistics about the loaded data with task-specific metrics."""
        stats = super().get_data_stats(data_points)
        
        if data_points:
            # Add task-specific statistics
            task_counts = [len(p.detected_tasks) for p in data_points]
            stats.update({
                'task_counts': task_counts,
                'avg_tasks': sum(task_counts) / len(task_counts),
                'min_tasks': min(task_counts),
                'max_tasks': max(task_counts)
            })
        
        return stats
    
    def _extract_screen_text(self, data_point: EvalDataPoint) -> str:
        """Extract screen text from data point."""
        if data_point.current_state and 'data' in data_point.current_state:
            text_parts = []
            for item in data_point.current_state['data']:
                if 'text_content' in item:
                    text_parts.extend(item['text_content'])
            return "\n".join(text_parts) if text_parts else "No screen text available"
        return "No screen text available"
    
    def _extract_active_url(self, data_point: EvalDataPoint) -> str:
        """Extract active URL from data point."""
        if data_point.current_state and 'data' in data_point.current_state:
            for item in data_point.current_state['data']:
                if item.get('application_name') == 'Brave Browser' and 'text_content' in item:
                    if item['text_content']:
                        return item['text_content'][0]
        return "No URL available"
    
    def filter_data(self, data_points: List[EvalDataPoint], **filters) -> List[EvalDataPoint]:
        """Filter data points based on criteria."""
        filtered = []
        
        for point in data_points:
            include = True
            
            # Filter by metadata
            if 'metadata' in filters:
                meta_filters = filters['metadata']
                for key, value in meta_filters.items():
                    if point.metadata.get(key) != value:
                        include = False
                        break
            
            # Filter by task count
            if 'min_tasks' in filters:
                if len(point.detected_tasks) < filters['min_tasks']:
                    include = False
            
            if 'max_tasks' in filters:
                if len(point.detected_tasks) > filters['max_tasks']:
                    include = False
            
            # Filter by summary presence
            if 'has_summary' in filters:
                if bool(point.summary) != filters['has_summary']:
                    include = False
            
            # Filter by state completeness
            if 'has_prev_state' in filters:
                if bool(point.prev_state) != filters['has_prev_state']:
                    include = False
            
            if include:
                filtered.append(point)
        
        return filtered
