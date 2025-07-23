"""
Task Detection Evaluation Package

A comprehensive evaluation system for task detection functionality.
"""

# Package version
__version__ = "1.0.0"

# Make main components available at package level
try:
    from .evaluate import TaskDetectionEvaluator, TaskDetectionResult
    from .visualize import EvalVisualizer
    
    __all__ = ['TaskDetectionEvaluator', 'TaskDetectionResult', 'EvalVisualizer']
except ImportError:
    # Handle case where dependencies aren't installed
    __all__ = []
