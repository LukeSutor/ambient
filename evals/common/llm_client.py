"""
LLM Client with automatic server management for evaluations.
"""
import os
import subprocess
import time
import requests
import psutil
import yaml
import json
from typing import Dict, Any, Optional, List
import logging

logger = logging.getLogger(__name__)

class LLMClient:
    def __init__(self, config_path: str = "config.yaml"):
        """Initialize LLM client with configuration."""
        with open(config_path, 'r') as f:
            self.config = yaml.safe_load(f)
        
        self.server_config = self.config['server']
        self.model_config = self.config['model']
        self.generation_config = self.config['generation']
        
        self.base_url = f"http://{self.server_config['host']}:{self.server_config['port']}"
        self.server_process = None
        self._session_params = {}
        
    def _find_server_process(self) -> Optional[psutil.Process]:
        """Find existing llama.cpp server process."""
        for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
            try:
                # Check if process name contains llama-server
                if proc.info['name'] and 'llama-server' in proc.info['name']:
                    return proc
                
                # Check command line args if they exist
                cmdline = proc.info['cmdline']
                if cmdline and any('llama-server' in str(arg) for arg in cmdline):
                    return proc
                    
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                continue
        return None
        
    def _start_server(self) -> bool:
        """Start the llama.cpp server if not running."""
        # Check if server is already running
        if self._is_server_running():
            logger.info("Server already running")
            return True
            
        # Check for existing process
        existing_proc = self._find_server_process()
        if existing_proc:
            logger.info(f"Found existing server process: {existing_proc.pid}")
            self.server_process = existing_proc
            return True
            
        # Start new server
        model_path = self.model_config['path']
        server_exe = self.server_config['executable']
        
        cmd = [
            server_exe,
            "-m", model_path,
            "--host", self.server_config['host'],
            "--port", str(self.server_config['port']),
            "--ctx-size", str(self.model_config['context_size']),
            "--n-gpu-layers", str(self.model_config['gpu_layers'])
        ]
        
        try:
            logger.info(f"Starting server: {' '.join(cmd)}")
            self.server_process = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                cwd=os.path.dirname(server_exe)
            )
            
            # Wait for server to start
            for _ in range(30):  # 30 second timeout
                if self._is_server_running():
                    logger.info("Server started successfully")
                    return True
                time.sleep(1)
                
            logger.error("Server failed to start within timeout")
            return False
            
        except Exception as e:
            logger.error(f"Failed to start server: {e}")
            return False
    
    def _is_server_running(self) -> bool:
        """Check if server is responding."""
        try:
            response = requests.get(f"{self.base_url}/health", timeout=2)
            return response.status_code == 200
        except:
            return False
    
    def _stop_server(self):
        """Stop the server if we started it."""
        if self.server_process and hasattr(self.server_process, 'terminate'):
            try:
                self.server_process.terminate()
                self.server_process.wait(timeout=5)
                logger.info("Server stopped")
            except:
                try:
                    self.server_process.kill()
                    logger.info("Server force killed")
                except:
                    logger.error("Failed to stop server")
    
    def generate(self, prompt: str, schema: Optional[Dict[str, Any]] = None, **kwargs) -> str:
        """Generate text using the LLM with optional JSON schema constraint."""
        if not self._is_server_running():
            if not self._start_server():
                raise RuntimeError("Failed to start LLM server")
        
        # Merge session params with generation config and kwargs
        params = {**self.generation_config, **self._session_params, **kwargs}
        
        payload = {
            "prompt": prompt,
            "stream": False,
            **params
        }
        
        # Add JSON schema if provided
        if schema:
            payload["response_format"] = {
                "type": "json_object",
                "schema": schema
            }
        
        try:
            response = requests.post(
                f"{self.base_url}/completion",
                json=payload,
                timeout=self.server_config['timeout']
            )
            response.raise_for_status()
            
            result = response.json()
            return result.get('content', '').strip()
            
        except Exception as e:
            logger.error(f"Generation failed: {e}")
            raise
    
    def generate_structured(self, prompt: str, schema: Dict[str, Any], **kwargs) -> Dict[str, Any]:
        """Generate structured JSON response using schema constraint."""
        response_text = self.generate(prompt, schema=schema, **kwargs)
        
        try:
            # Parse JSON response
            return json.loads(response_text)
        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse structured response: {e}")
            logger.error(f"Response text: {response_text}")
            raise ValueError(f"Invalid JSON response: {e}")
    
    def set_session_params(self, **params):
        """Set parameters that persist for this session."""
        self._session_params.update(params)
    
    def clear_session_params(self):
        """Clear session parameters."""
        self._session_params.clear()
    
    def __enter__(self):
        """Context manager entry."""
        if self.server_config.get('auto_start', True):
            self._start_server()
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        if self.server_config.get('auto_stop', True):
            self._stop_server()
