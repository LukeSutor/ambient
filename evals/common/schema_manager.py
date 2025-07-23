"""
JSON Schema management for evaluation system.
Handles loading, saving, and validating JSON schemas for LLM inference.
"""
import json
import os
import yaml
from typing import Dict, Any, Optional
import logging

logger = logging.getLogger(__name__)

class SchemaManager:
    """Manages JSON schemas for evaluation prompts and responses."""
    
    def __init__(self, config_path: str = "config.yaml"):
        """Initialize schema manager with configuration."""
        with open(config_path, 'r') as f:
            self.config = yaml.safe_load(f)
        
        self.schema_config = self.config.get('schemas', {})
        self.schema_dir = self.schema_config.get('schema_dir', './schemas')
        self.schemas = {}
        
        # Create schema directory if it doesn't exist
        os.makedirs(self.schema_dir, exist_ok=True)
        
        # Load schema files first (prioritize over config)
        self._load_schema_files()
        
        # Load schemas from config as fallback
        self._load_schemas_from_config()
    
    def _load_schemas_from_config(self):
        """Load schema definitions from configuration."""
        for eval_type, schemas in self.schema_config.items():
            if eval_type == 'schema_dir':
                continue
                
            if not isinstance(schemas, dict):
                continue
                
            for schema_name, schema_content in schemas.items():
                if isinstance(schema_content, str):
                    try:
                        # Parse YAML/JSON schema content
                        schema_obj = yaml.safe_load(schema_content)
                        schema_key = f"{eval_type}.{schema_name}"
                        self.schemas[schema_key] = schema_obj
                        logger.info(f"Loaded schema: {schema_key}")
                    except Exception as e:
                        logger.error(f"Failed to parse schema {eval_type}.{schema_name}: {e}")
    
    def _load_schema_files(self):
        """Load schema files from the schema directory."""
        if not os.path.exists(self.schema_dir):
            return
            
        for filename in os.listdir(self.schema_dir):
            if filename.endswith('.json'):
                filepath = os.path.join(self.schema_dir, filename)
                schema_key = filename.replace('.json', '')
                
                try:
                    with open(filepath, 'r') as f:
                        schema_obj = json.load(f)
                    self.schemas[schema_key] = schema_obj
                    logger.info(f"Loaded schema file: {schema_key}")
                except Exception as e:
                    logger.error(f"Failed to load schema file {filepath}: {e}")
    
    def get_schema(self, schema_key: str) -> Optional[Dict[str, Any]]:
        """Get a schema by its key."""
        return self.schemas.get(schema_key)
    
    def save_schema(self, schema_key: str, schema_obj: Dict[str, Any]) -> str:
        """Save a schema to a JSON file."""
        filepath = os.path.join(self.schema_dir, f"{schema_key}.json")
        
        try:
            with open(filepath, 'w') as f:
                json.dump(schema_obj, f, indent=2)
            
            # Also update in-memory cache
            self.schemas[schema_key] = schema_obj
            
            logger.info(f"Saved schema: {schema_key} to {filepath}")
            return filepath
            
        except Exception as e:
            logger.error(f"Failed to save schema {schema_key}: {e}")
            raise
    
    def validate_response(self, response_data: Any, schema_key: str) -> bool:
        """Validate a response against a schema."""
        schema = self.get_schema(schema_key)
        if not schema:
            logger.warning(f"Schema not found: {schema_key}")
            return False
        
        try:
            # Basic validation - could use jsonschema library for full validation
            if isinstance(response_data, str):
                response_data = json.loads(response_data)
            
            # Simple validation for required fields
            required_fields = schema.get('properties', {}).get('required', [])
            if isinstance(response_data, dict):
                for field in required_fields:
                    if field not in response_data:
                        logger.error(f"Missing required field: {field}")
                        return False
            
            return True
            
        except Exception as e:
            logger.error(f"Validation failed for {schema_key}: {e}")
            return False
    
    def get_schema_for_prompt(self, eval_type: str, prompt_name: str) -> Optional[Dict[str, Any]]:
        """Get the appropriate schema for a given evaluation type and prompt."""
        schema_key = f"{eval_type}.{prompt_name}"
        return self.get_schema(schema_key)
    
    def list_schemas(self) -> Dict[str, Dict[str, Any]]:
        """List all available schemas."""
        return self.schemas.copy()
    
    def export_schemas_to_rust(self, output_path: str = "schemas_export.rs"):
        """Export schemas in Rust format for integration with the main app."""
        rust_content = [
            "use once_cell::sync::Lazy;",
            "use std::collections::HashMap;",
            "",
            "// Auto-generated schema definitions for evaluation system",
            "static EVAL_SCHEMAS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {",
            "  let mut map = HashMap::new();"
        ]
        
        for schema_key, schema_obj in self.schemas.items():
            schema_json = json.dumps(schema_obj, indent=2)
            # Escape quotes for Rust string literal
            escaped_json = schema_json.replace('"', '\\"').replace('\n', '\\n')
            
            rust_content.append(f'  map.insert(')
            rust_content.append(f'    "{schema_key}",')
            rust_content.append(f'    r#"{escaped_json}"#,')
            rust_content.append('  );')
        
        rust_content.extend([
            "  map",
            "});",
            "",
            "/// Get evaluation schema by key",
            "pub fn get_eval_schema(key: &str) -> Option<&'static str> {",
            "  EVAL_SCHEMAS.get(key).copied()",
            "}",
            "",
            "/// Tauri command to get evaluation schema",
            "#[tauri::command]",
            "pub fn get_eval_schema_command(key: String) -> Result<String, String> {",
            "  match get_eval_schema(&key) {",
            "    Some(schema) => Ok(schema.to_string()),",
            "    None => Err(format!(\"Evaluation schema '{}' not found.\", key)),",
            "  }",
            "}"
        ])
        
        with open(output_path, 'w') as f:
            f.write('\n'.join(rust_content))
        
        logger.info(f"Exported schemas to Rust format: {output_path}")
        return output_path
