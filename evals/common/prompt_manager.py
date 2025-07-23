"""
YAML-based prompt management system for evaluations.
"""
import os
import yaml
from typing import Dict, Any, List, Optional
import logging

logger = logging.getLogger(__name__)

class PromptManager:
    def __init__(self, prompts_dir: str = "prompts"):
        """Initialize prompt manager with prompts directory."""
        self.prompts_dir = prompts_dir
        self.prompts_cache = {}
        self._load_all_prompts()
    
    def _load_all_prompts(self):
        """Load all YAML prompt files into cache."""
        if not os.path.exists(self.prompts_dir):
            logger.warning(f"Prompts directory not found: {self.prompts_dir}")
            return
            
        for filename in os.listdir(self.prompts_dir):
            if filename.endswith('.yaml') or filename.endswith('.yml'):
                filepath = os.path.join(self.prompts_dir, filename)
                eval_type = filename.replace('.yaml', '').replace('.yml', '')
                
                try:
                    with open(filepath, 'r', encoding='utf-8') as f:
                        self.prompts_cache[eval_type] = yaml.safe_load(f)
                    logger.info(f"Loaded prompts for: {eval_type}")
                except Exception as e:
                    logger.error(f"Failed to load {filepath}: {e}")
    
    def get_prompt(self, eval_type: str, prompt_name: str, **kwargs) -> str:
        """Get a formatted prompt by eval type and name."""
        if eval_type not in self.prompts_cache:
            raise ValueError(f"Eval type '{eval_type}' not found. Available: {list(self.prompts_cache.keys())}")
        
        eval_prompts = self.prompts_cache[eval_type]
        
        if prompt_name not in eval_prompts:
            raise ValueError(f"Prompt '{prompt_name}' not found in {eval_type}. Available: {list(eval_prompts.keys())}")
        
        prompt_data = eval_prompts[prompt_name]
        
        # Handle both string and dict prompt formats
        if isinstance(prompt_data, str):
            prompt_template = prompt_data
        elif isinstance(prompt_data, dict):
            prompt_template = prompt_data.get('template', prompt_data.get('prompt', ''))
        else:
            raise ValueError(f"Invalid prompt format for {eval_type}.{prompt_name}")
        
        # Format the prompt with provided kwargs
        try:
            return prompt_template.format(**kwargs)
        except KeyError as e:
            raise ValueError(f"Missing template variable {e} for prompt {eval_type}.{prompt_name}")
    
    def get_prompt_info(self, eval_type: str, prompt_name: str) -> Dict[str, Any]:
        """Get full prompt information including metadata."""
        if eval_type not in self.prompts_cache:
            raise ValueError(f"Eval type '{eval_type}' not found")
        
        eval_prompts = self.prompts_cache[eval_type]
        
        if prompt_name not in eval_prompts:
            raise ValueError(f"Prompt '{prompt_name}' not found in {eval_type}")
        
        prompt_data = eval_prompts[prompt_name]
        
        if isinstance(prompt_data, str):
            return {
                'template': prompt_data,
                'description': None,
                'examples': [],
                'parameters': []
            }
        elif isinstance(prompt_data, dict):
            return prompt_data
        else:
            raise ValueError(f"Invalid prompt format for {eval_type}.{prompt_name}")
    
    def list_eval_types(self) -> List[str]:
        """List all available evaluation types."""
        return list(self.prompts_cache.keys())
    
    def list_prompts(self, eval_type: str) -> List[str]:
        """List all prompts for a given evaluation type."""
        if eval_type not in self.prompts_cache:
            raise ValueError(f"Eval type '{eval_type}' not found")
        return list(self.prompts_cache[eval_type].keys())
    
    def reload_prompts(self):
        """Reload all prompts from disk."""
        self.prompts_cache.clear()
        self._load_all_prompts()
    
    def validate_prompt(self, eval_type: str, prompt_name: str, **test_kwargs) -> bool:
        """Validate that a prompt can be formatted with given kwargs."""
        try:
            self.get_prompt(eval_type, prompt_name, **test_kwargs)
            return True
        except Exception as e:
            logger.error(f"Prompt validation failed: {e}")
            return False
