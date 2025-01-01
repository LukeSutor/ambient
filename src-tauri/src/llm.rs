use llama_cpp_rs::{
    options::{ModelOptions, PredictOptions},
    LLama,
};
use std::sync::{Arc, Mutex};

pub struct LLMService {
    llama: Arc<Mutex<LLama>>,
}

impl LLMService {
    pub fn new(model_path: String) -> Result<Self, String> {
        let model_options = ModelOptions::default();
        let llama = LLama::new(model_path, &model_options)
            .map_err(|e| e.to_string())?;
            
        Ok(Self {
            llama: Arc::new(Mutex::new(llama))
        })
    }

    pub async fn generate(&self, prompt: String) -> Result<String, String> {
        let predict_options = PredictOptions {
            token_callback: Some(Box::new(|token| {
                println!("Generated token: {}", token);
                true
            })),
            ..Default::default()
        };

        self.llama
            .lock()
            .map_err(|e| e.to_string())?
            .predict(prompt, predict_options)
            .map_err(|e| e.to_string())
    }
}