"""
Data loading utilities for evaluation data.
"""
import json
import os
import glob
from typing import List, Dict, Any, Optional, Iterator
from dataclasses import dataclass
import logging

logger = logging.getLogger(__name__)

@dataclass
class EvalDataPoint:
    """Single evaluation data point."""
    id: str
    timestamp: str
    prev_prev_state: Optional[Dict[str, Any]]
    prev_state: Optional[Dict[str, Any]]
    current_state: Dict[str, Any]
    screen_diff: Optional[Dict[str, Any]]
    detected_tasks: List[Dict[str, Any]]
    summary: Optional[str]
    metadata: Dict[str, Any]
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'EvalDataPoint':
        """Create EvalDataPoint from dictionary."""
        return cls(
            id=data.get('id', ''),
            timestamp=data.get('timestamp', ''),
            prev_prev_state=data.get('prev_prev_state'),
            prev_state=data.get('prev_state'),
            current_state=data.get('current_state', {}),
            screen_diff=data.get('screen_diff'),
            detected_tasks=data.get('detected_tasks', []),
            summary=data.get('summary'),
            metadata=data.get('metadata', {})
        )

class DataLoader:
    """Load and manage evaluation data."""
    
    def __init__(self, data_dir: str = "data"):
        """Initialize data loader with data directory."""
        self.data_dir = data_dir
        
    def load_json_file(self, filepath: str) -> EvalDataPoint:
        """Load a single JSON eval data file."""
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                data = json.load(f)
            return EvalDataPoint.from_dict(data)
        except Exception as e:
            logger.error(f"Failed to load {filepath}: {e}")
            raise
    
    def load_all_data(self, pattern: str = "*.json") -> List[EvalDataPoint]:
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
                data_points.append(data_point)
            except Exception as e:
                logger.error(f"Skipping {filepath}: {e}")
                continue
        
        return data_points
    
    def load_data_batch(self, pattern: str = "*.json", batch_size: int = 10) -> Iterator[List[EvalDataPoint]]:
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
    
    def get_data_stats(self, data_points: List[EvalDataPoint]) -> Dict[str, Any]:
        """Get statistics about the loaded data."""
        if not data_points:
            return {}
        
        stats = {
            'total_count': len(data_points),
            'with_summary': sum(1 for p in data_points if p.summary),
            'with_prev_state': sum(1 for p in data_points if p.prev_state),
            'with_screen_diff': sum(1 for p in data_points if p.screen_diff),
            'task_counts': [len(p.detected_tasks) for p in data_points],
            'metadata_keys': set()
        }
        
        # Collect all metadata keys
        for point in data_points:
            stats['metadata_keys'].update(point.metadata.keys())
        
        stats['metadata_keys'] = list(stats['metadata_keys'])
        
        # Task count statistics
        if stats['task_counts']:
            stats['avg_tasks'] = sum(stats['task_counts']) / len(stats['task_counts'])
            stats['min_tasks'] = min(stats['task_counts'])
            stats['max_tasks'] = max(stats['task_counts'])
        
        return stats
    
    def save_data(self, data_point: EvalDataPoint, filename: Optional[str] = None) -> str:
        """Save a data point to JSON file."""
        if not os.path.exists(self.data_dir):
            os.makedirs(self.data_dir)
        
        if filename is None:
            filename = f"eval_data_{data_point.id}.json"
        
        filepath = os.path.join(self.data_dir, filename)
        
        # Convert dataclass to dict
        data_dict = {
            'id': data_point.id,
            'timestamp': data_point.timestamp,
            'prev_prev_state': data_point.prev_prev_state,
            'prev_state': data_point.prev_state,
            'current_state': data_point.current_state,
            'screen_diff': data_point.screen_diff,
            'detected_tasks': data_point.detected_tasks,
            'summary': data_point.summary,
            'metadata': data_point.metadata
        }
        
        with open(filepath, 'w', encoding='utf-8') as f:
            json.dump(data_dict, f, indent=2, ensure_ascii=False)
        
        logger.info(f"Saved data to: {filepath}")
        return filepath
