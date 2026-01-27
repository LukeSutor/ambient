use crate::setup::{get_embedding_model_path, get_embedding_tokenizer_path};
use rten::Model;
use rten_tensor::prelude::*;
use rten_tensor::{NdTensorView, Tensor};
use rten_text::tokenizer::{EncodeOptions, Tokenizer};
use tauri::AppHandle;

#[tauri::command]
pub async fn generate_embedding(app_handle: AppHandle, input: String) -> Result<Vec<f32>, String> {
  log::info!("[Embedding] Generating embedding from ONNX model");
  let input = input.trim().to_string();

  let model_path = get_embedding_model_path(&app_handle)
    .map_err(|e| format!("Failed to get embedding model path: {}", e))?;
  
  let tokenizer_path = get_embedding_tokenizer_path(&app_handle)
    .map_err(|e| format!("Failed to get tokenizer path: {}", e))?;

  // Load model and tokenizer
  let model = Model::load_file(&model_path)
    .map_err(|e| format!("Failed to load embedding model: {}", e))?;
  
  let tokenizer = Tokenizer::from_file(&tokenizer_path)
    .map_err(|e| format!("Failed to load tokenizer: {}", e))?;

  // Tokenize input
  let encoded = tokenizer
    .encode(&input, Some(EncodeOptions::default()))
    .map_err(|e| format!("Tokenization failed: {}", e))?;

  let token_ids: Vec<i32> = encoded.token_ids().iter().map(|&id| id as i32).collect();
  let n_tokens = token_ids.len();

  // Create input tensors
  let input_ids = Tensor::from_vec(token_ids).into_shape([1, n_tokens]);
  let attention_mask = Tensor::full(&[1, n_tokens], 1i32);
  let token_type_ids = Tensor::full(&[1, n_tokens], 0i32);

  // Get input node IDs
  let input_ids_id = model
    .node_id("input_ids")
    .map_err(|_| "Model missing 'input_ids' input")?;
  let attention_mask_id = model
    .node_id("attention_mask")
    .map_err(|_| "Model missing 'attention_mask' input")?;
  
  // Some models (like BERT/BGE) require token_type_ids, others (like Gemma) don't
  let token_type_ids_id = model.node_id("token_type_ids");

  let mut inputs = vec![
    (input_ids_id, input_ids.view().into()),
    (attention_mask_id, attention_mask.view().into()),
  ];

  if let Ok(id) = token_type_ids_id {
    inputs.push((id, token_type_ids.view().into()));
  }

  // Run model
  let output_id = model
    .find_node("sentence_embedding")
    .ok_or("Could not find output node in model")?;

  let outputs = model
    .run_n(inputs, [output_id], None)
    .map_err(|e| format!("Model execution failed: {}", e))?;

  // Convert output (batch, embed_dim) to vector.
  let output_2d: NdTensorView<f32, 2> = outputs[0]
    .as_view()
    .try_into()
    .map_err(|_| format!("Expected rank 2 output [batch, dim], got {:?}", outputs[0].shape().to_vec()))?;

  let mut mean_embedding: Vec<f32> = output_2d
    .to_tensor()
    .data()
    .ok_or("Failed to get tensor data")?
    .to_vec();

  // L2 Normalize the embedding
  let norm = mean_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
  if norm > 0.0 {
    for val in mean_embedding.iter_mut() {
      *val /= norm;
    }
  }

  Ok(mean_embedding)
}
