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
    ground_truth: list | str
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