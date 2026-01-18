use crate::db::core::DbState;
use chrono::{Utc, DateTime, Datelike, NaiveDate, NaiveDateTime};
use rusqlite::params;
use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::collections::{HashSet, BTreeMap};
use crate::constants::{COST_PER_TOKEN, WATER_PER_TOKEN, ENERGY_PER_TOKEN};

/// Time filters for querying token usage
#[derive(Debug, Serialize, Deserialize, TS, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "token_usage.ts")]
pub enum TimeFilter {
  Last24Hours,
  Last7Days,
  Last30Days,
  LastYear,
  AllTime,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "token_usage.ts")]
pub enum AggregationLevel {
  Hour,
  Day,
  Week,
  Month,
}

/// Internal struct for SQL data mapping
pub struct TokenUsageRow {
  pub model: String,
  pub total_prompt_tokens: u32,
  pub total_completion_tokens: u32,
  pub date: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "token_usage.ts")]
pub struct TokenUsageQueryResult {
  pub time_filter: TimeFilter,
  pub aggregation_level: AggregationLevel,
  #[ts(type = "({ time_label: string, date: string } & Record<string, number>)[]")]
  pub data: Vec<serde_json::Value>,
  pub models: Vec<String>,
  pub total_prompt_tokens: u32,
  pub total_completion_tokens: u32,
  pub time_range: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "token_usage.ts")]
pub struct TokenUsageConsumptionResult {
  pub cost_amount: f64,
  pub cost_unit: String,
  pub water_amount: f64,
  pub water_unit: String,
  pub energy_amount: f64,
  pub energy_unit: String,
}

/// Add token usage record
pub async fn add_token_usage(
  app_handle: AppHandle,
  model: &str,
  prompt_tokens: u32,
  completion_tokens: u32,
) -> Result<(), String> {
  let state = app_handle.state::<DbState>();
  let db_guard = state.0.lock().unwrap();
  let conn = db_guard
    .as_ref()
    .ok_or("Database connection not available")?;

  let now = Utc::now();

  // Get the model ID
  let model_id: i64 = conn
    .query_row(
      "SELECT id FROM models WHERE model = ?1",
      params![model],
      |row| row.get(0),
    )
    .map_err(|e| format!("Failed to get model ID: {}", e))?;

  // Insert the message
  conn
    .execute(
      "INSERT INTO token_usage (model, prompt_tokens, completion_tokens, timestamp)
         VALUES (?1, ?2, ?3, ?4)",
      params![
        model_id,
        prompt_tokens,
        completion_tokens,
        now.to_rfc3339(),
      ],
    )
    .map_err(|e| format!("Failed to add message: {}", e))?;
  
  Ok(())
}

/// Get total token usage for a model
pub async fn get_total_token_usage(
  app_handle: AppHandle,
  model: &str,
) -> Result<(u32, u32), String> {
  let state = app_handle.state::<DbState>();
  let db_guard = state.0.lock().unwrap();
  let conn = db_guard
    .as_ref()
    .ok_or("Database connection not available")?;

  // Get the model ID
  let model_id: i64 = conn
    .query_row(
      "SELECT id FROM models WHERE model = ?1",
      params![model],
      |row| row.get(0),
    )
    .map_err(|e| format!("Failed to get model ID: {}", e))?;

  // Query total token usage
  let mut stmt = conn
    .prepare(
      "SELECT SUM(prompt_tokens), SUM(completion_tokens)
         FROM token_usage
         WHERE model = ?1",
    )
    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

  let (total_prompt_tokens, total_completion_tokens): (Option<u32>, Option<u32>) =
    stmt
      .query_row(params![model_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
      })
      .map_err(|e| format!("Failed to query token usage: {}", e))?;

  Ok((
    total_prompt_tokens.unwrap_or(0),
    total_completion_tokens.unwrap_or(0),
  ))
}

/// Get the estimated cost, water, and energy savings for the local token counts
#[tauri::command]
pub async fn get_token_usage_consumption(app_handle: AppHandle) -> Result<TokenUsageConsumptionResult, String> {
  // Get total token usage for local mode
  let (total_prompt_tokens, total_completion_tokens) = get_total_token_usage(app_handle, "local").await?;
  let total_tokens = total_prompt_tokens + total_completion_tokens;

  // Calculate estimates based on constants
  let cost_amount = (total_tokens as f64) * COST_PER_TOKEN;
  let water_amount = (total_tokens as f64) * WATER_PER_TOKEN;
  let energy_amount = (total_tokens as f64) * ENERGY_PER_TOKEN;

  // Determine units based on magnitude
  // Dollars or cents for USD
  let (cost_amount, cost_unit) = if cost_amount >= 1.0 {
    (cost_amount, "$".to_string())
  } else {
    (cost_amount * 100.0, "¢".to_string())
  };

  // Liters or milliliters for water
  let (water_amount, water_unit) = if water_amount >= 1000.0 {
    (water_amount / 1000.0, "L".to_string())
  } else {
    (water_amount, "mL".to_string())
  };

  // kWh or Wh for energy
  let (energy_amount, energy_unit) = if energy_amount >= 1000.0 {
    (energy_amount / 1000.0, "kWh".to_string())
  } else {
    (energy_amount, "Wh".to_string())
  };
  Ok(TokenUsageConsumptionResult {
    cost_amount,
    cost_unit,
    water_amount,
    water_unit,
    energy_amount,
    energy_unit,
  })
}

/// Get token usage aggregated by the specified level and model filtered by time range
#[tauri::command]
pub async fn get_token_usage(
  app_handle: AppHandle,
  time_filter: TimeFilter,
  aggregation_level: AggregationLevel,
) -> Result<TokenUsageQueryResult, String> {
  // 1. Validation
  if aggregation_level == AggregationLevel::Hour && time_filter != TimeFilter::Last24Hours {
    return Err("Hourly aggregation is only allowed for the Last 24 Hours time filter.".into());
  }
  if aggregation_level == AggregationLevel::Day
    && (time_filter == TimeFilter::LastYear || time_filter == TimeFilter::AllTime)
  {
    return Err("Daily aggregation is not allowed for Last Year or All Time filtering.".into());
  }

  let state = app_handle.state::<DbState>();
  let db_guard = state.0.lock().unwrap();
  let conn = db_guard
    .as_ref()
    .ok_or("Database connection not available")?;

  // Determine the time range based on the filter
  let time_condition = match time_filter {
    TimeFilter::Last24Hours => "timestamp >= datetime('now', '-1 day')",
    TimeFilter::Last7Days => "timestamp >= datetime('now', '-7 days')",
    TimeFilter::Last30Days => "timestamp >= datetime('now', '-30 days')",
    TimeFilter::LastYear => "timestamp >= datetime('now', '-1 year')",
    TimeFilter::AllTime => "1=1", // No time filter
  };

  // Determine the grouping based on aggregation level
  let sql_group = match aggregation_level {
    AggregationLevel::Hour => "strftime('%Y-%m-%dT%H:00:00', tu.timestamp)",
    AggregationLevel::Day => "DATE(tu.timestamp)",
    AggregationLevel::Week => "date(tu.timestamp, 'weekday 0', '-6 days')",
    AggregationLevel::Month => "strftime('%Y-%m-01', tu.timestamp)",
  };

  // Query token usage aggregated by specified level and model
  let mut stmt = conn
    .prepare(&format!(
      "SELECT m.model, {} as period,
              SUM(tu.prompt_tokens) as total_prompt_tokens,
              SUM(tu.completion_tokens) as total_completion_tokens
         FROM token_usage tu
         JOIN models m ON tu.model = m.id
         WHERE {}
         GROUP BY m.model, period
         ORDER BY period ASC",
      sql_group, time_condition
    ))
    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

  let token_usage_iter = stmt
    .query_map([], |row| {
      Ok(TokenUsageRow {
        model: row.get(0)?,
        date: row.get(1)?,
        total_prompt_tokens: row.get(2)?,
        total_completion_tokens: row.get(3)?,
      })
    })
    .map_err(|e| format!("Failed to query token usage: {}", e))?;

  // Helper for parsing dates from SQL formats
  let parse_date = |s: &str| -> Option<DateTime<Utc>> {
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
      return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
      let dt = d.and_hms_opt(0, 0, 0)?;
      return Some(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    None
  };

  let mut rows_with_dates = Vec::new();
  let mut years = HashSet::new();
  let mut models_set = HashSet::new();
  let mut min_date: Option<DateTime<Utc>> = None;
  let mut max_date: Option<DateTime<Utc>> = None;

  let mut total_prompt_tokens = 0;
  let mut total_completion_tokens = 0;

  for res in token_usage_iter {
    let row = res.map_err(|e| format!("Failed to read row: {}", e))?;
    if let Some(dt) = parse_date(&row.date) {
      years.insert(dt.year());
      models_set.insert(row.model.clone());
      if min_date.map_or(true, |m| dt < m) {
        min_date = Some(dt);
      }
      if max_date.map_or(true, |m| dt > m) {
        max_date = Some(dt);
      }
      total_prompt_tokens += row.total_prompt_tokens;
      total_completion_tokens += row.total_completion_tokens;
      rows_with_dates.push((row, dt));
    }
  }

  let multi_year = years.len() > 1;
  let mut data_points_map: BTreeMap<String, serde_json::Value> = BTreeMap::new();

  for (row, dt) in rows_with_dates {
    let entry = data_points_map.entry(row.date.clone()).or_insert_with(|| {
      let time_label = match aggregation_level {
        AggregationLevel::Hour => dt.format("%H:00").to_string(),
        AggregationLevel::Day => {
          if multi_year {
            dt.format("%b %d, %y").to_string()
          } else {
            dt.format("%b %d").to_string()
          }
        }
        AggregationLevel::Week => {
          if multi_year {
            format!("{}", dt.format("%b %d, %y"))
          } else {
            format!("{}", dt.format("%b %d"))
          }
        }
        AggregationLevel::Month => {
          if multi_year {
            dt.format("%b %y'").to_string()
          } else {
            dt.format("%b").to_string()
          }
        }
      };
      serde_json::json!({
        "time_label": time_label,
        "date": row.date.clone(),
      })
    });

    let total = row.total_prompt_tokens + row.total_completion_tokens;
    if let Some(obj) = entry.as_object_mut() {
      let current = obj
        .get(&row.model)
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
      obj.insert(row.model.clone(), serde_json::json!(current + total as u64));
    }
  }

  let final_data: Vec<serde_json::Value> = data_points_map.into_values().collect();
  let mut models_list: Vec<String> = models_set.into_iter().collect();
  models_list.sort();

  let time_range = if let (Some(min_dt), Some(max_dt)) = (min_date, max_date) {
    if min_dt.year() == max_dt.year() {
      if min_dt.month() == max_dt.month() {
        if min_dt.day() == max_dt.day() {
          min_dt.format("%B %d, %Y").to_string()
        } else {
          format!(
            "{} - {}, {}",
            min_dt.format("%B %d"),
            max_dt.format("%d"),
            min_dt.year()
          )
        }
      } else {
        format!(
          "{} - {} {}",
          min_dt.format("%B"),
          max_dt.format("%B"),
          min_dt.year()
        )
      }
    } else {
      format!(
        "{} {} - {} {}",
        min_dt.format("%B"),
        min_dt.year(),
        max_dt.format("%B"),
        max_dt.year()
      )
    }
  } else {
    "No data available".to_string()
  };

  Ok(TokenUsageQueryResult {
    time_filter,
    aggregation_level,
    data: final_data,
    models: models_list,
    total_prompt_tokens,
    total_completion_tokens,
    time_range,
  })
}