use crate::db::DbState;
use crate::tasks::detection::TaskDetectionService;
use crate::tasks::models::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Result as SqliteResult};

pub struct TaskService;

impl TaskService {
  /// Create a new task with its steps
  pub fn create_task(
    db_state: &DbState,
    request: CreateTaskRequest,
  ) -> Result<TaskWithSteps, String> {
    let mut db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_mut()
      .ok_or("Database connection not available")?;

    // Start transaction
    let tx = conn
      .unchecked_transaction()
      .map_err(|e| format!("Transaction error: {}", e))?;

    // Calculate first scheduled date (current time for new tasks)
    let frequency_enum = &request.frequency;
    let first_scheduled_at = Utc::now();

    // Insert task
    let task_id = tx
            .prepare("INSERT INTO tasks (name, description, category, priority, frequency, first_scheduled_at, status) VALUES (?, ?, ?, ?, ?, ?, 'pending')")
            .and_then(|mut stmt| {
                stmt.insert(params![
                    request.name,
                    request.description,
                    request.category,
                    request.priority,
                    frequency_enum.as_str(),
                    first_scheduled_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ])
            })
            .map_err(|e| format!("Failed to insert task: {}", e))?;

    // Insert task steps
    let mut steps = Vec::new();
    for (index, step_request) in request.steps.iter().enumerate() {
      let step_id = tx
                .prepare("INSERT INTO task_steps (task_id, step_number, title, description, status) VALUES (?, ?, ?, ?, 'pending')")
                .and_then(|mut stmt| {
                    stmt.insert(params![
                        task_id,
                        index + 1,
                        step_request.title,
                        step_request.description,
                    ])
                })
                .map_err(|e| format!("Failed to insert task step: {}", e))?;

      steps.push(TaskStep {
        id: step_id,
        task_id,
        step_number: (index + 1) as i32,
        title: step_request.title.clone(),
        description: step_request.description.clone(),
        status: "pending".to_string(),
        completed_at: None,
      });
    }

    // Commit transaction
    tx.commit()
      .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    // Release the database lock before calling get_task_by_id
    drop(db_guard);

    // Get the created task
    let task = Self::get_task_by_id(db_state, task_id)?;

    Ok(TaskWithSteps {
      task,
      steps,
      progress_percentage: 0.0,
    })
  }

  /// Get task by ID
  pub fn get_task_by_id(db_state: &DbState, task_id: i64) -> Result<Task, String> {
    let db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_ref()
      .ok_or("Database connection not available")?;

    let mut stmt = conn
            .prepare("SELECT id, name, description, category, priority, frequency, last_completed_at, first_scheduled_at, created_at, updated_at, status FROM tasks WHERE id = ?")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let task = stmt
      .query_row(params![task_id], |row| {
        Ok(Task {
          id: row.get(0)?,
          name: row.get(1)?,
          description: row.get(2)?,
          category: row.get(3)?,
          priority: row.get(4)?,
          frequency: row.get(5)?,
          last_completed_at: row
            .get::<_, Option<String>>(6)?
            .map(|s| Self::parse_datetime(s)),
          first_scheduled_at: Self::parse_datetime(row.get::<_, String>(7)?),
          created_at: Self::parse_datetime(row.get::<_, String>(8)?),
          updated_at: Self::parse_datetime(row.get::<_, String>(9)?),
          status: row.get(10)?,
        })
      })
      .map_err(|e| format!("Task not found: {}", e))?;

    Ok(task)
  }

  /// Get task with all its steps
  pub fn get_task_with_steps(db_state: &DbState, task_id: i64) -> Result<TaskWithSteps, String> {
    let task = Self::get_task_by_id(db_state, task_id)?;
    let steps = Self::get_task_steps(db_state, task_id)?;
    let progress_percentage = Self::calculate_progress_percentage(&steps);

    Ok(TaskWithSteps {
      task,
      steps,
      progress_percentage,
    })
  }

  /// Get all steps for a task
  pub fn get_task_steps(db_state: &DbState, task_id: i64) -> Result<Vec<TaskStep>, String> {
    let db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_ref()
      .ok_or("Database connection not available")?;

    let mut stmt = conn
            .prepare("SELECT id, task_id, step_number, title, description, status, completed_at FROM task_steps WHERE task_id = ? ORDER BY step_number")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let steps = stmt
      .query_map(params![task_id], |row| {
        Ok(TaskStep {
          id: row.get(0)?,
          task_id: row.get(1)?,
          step_number: row.get(2)?,
          title: row.get(3)?,
          description: row.get(4)?,
          status: row.get(5)?,
          completed_at: row
            .get::<_, Option<String>>(6)?
            .map(|s| Self::parse_datetime(s)),
        })
      })
      .map_err(|e| format!("Failed to query steps: {}", e))?
      .collect::<SqliteResult<Vec<_>>>()
      .map_err(|e| format!("Failed to collect steps: {}", e))?;

    Ok(steps)
  }

  /// Get active (non-completed) steps for a task
  pub fn get_active_steps(db_state: &DbState, task_id: i64) -> Result<Vec<TaskStep>, String> {
    let steps = Self::get_task_steps(db_state, task_id)?;
    Ok(
      steps
        .into_iter()
        .filter(|step| step.status != "completed")
        .collect(),
    )
  }

  /// Get all active tasks (not completed)
  pub fn get_active_tasks(db_state: &DbState) -> Result<Vec<TaskWithSteps>, String> {
    let task_ids: Vec<i64> = {
      let db_guard = db_state
        .0
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
      let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

      let mut stmt = conn
        .prepare(
          "SELECT id FROM tasks WHERE status = 'pending' ORDER BY priority DESC, created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

      let task_ids: Vec<i64> = stmt
        .query_map([], |row| Ok(row.get(0)?))
        .map_err(|e| format!("Failed to query tasks: {}", e))?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| format!("Failed to collect task IDs: {}", e))?;

      task_ids
    }; // db_guard is dropped here

    let mut tasks = Vec::new();
    for task_id in task_ids {
      match Self::get_task_with_steps(db_state, task_id) {
        Ok(task_with_steps) => tasks.push(task_with_steps),
        Err(e) => log::error!("Failed to get task {}: {}", task_id, e),
      }
    }

    Ok(tasks)
  }

  /// Update step status
  pub fn update_step_status(
    db_state: &DbState,
    step_id: i64,
    status: StepStatus,
  ) -> Result<(), String> {
    let mut db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_mut()
      .ok_or("Database connection not available")?;

    let completed_at = if status == StepStatus::Completed {
      Some(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())
    } else {
      None
    };

    conn
      .execute(
        "UPDATE task_steps SET status = ?, completed_at = ? WHERE id = ?",
        params![status.as_str(), completed_at, step_id],
      )
      .map_err(|e| format!("Failed to update step status: {}", e))?;

    Ok(())
  }

  /// Record task progress from LLM analysis
  pub fn record_progress(
    db_state: &DbState,
    task_id: i64,
    step_id: Option<i64>,
    reasoning: Option<&str>,
  ) -> Result<(), String> {
    let mut db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_mut()
      .ok_or("Database connection not available")?;

    conn
      .execute(
        "INSERT INTO task_progress (task_id, step_id, reasoning) VALUES (?, ?, ?)",
        params![task_id, step_id, reasoning],
      )
      .map_err(|e| format!("Failed to record progress: {}", e))?;

    Ok(())
  }

  /// Analyze current screen for task progress
  pub async fn analyze_task_progress(
    db_state: &DbState,
    task_id: i64,
    screen_context: ScreenContext,
    llm_completion_fn: impl Fn(&str) -> Result<String, String>,
  ) -> Result<TaskProgressUpdate, String> {
    // Get active steps for the task
    let active_steps = Self::get_active_steps(db_state, task_id)?;

    if active_steps.is_empty() {
      return Ok(TaskProgressUpdate {
        task_id,
        step_updates: vec![],
        overall_status: TaskStatus::Completed,
      });
    }

    // Build prompt for LLM
    let prompt = TaskDetectionService::build_detection_prompt(&active_steps, &screen_context);

    // Get LLM response
    let llm_response = llm_completion_fn(&prompt)?;

    // Parse response
    let detection_result = TaskDetectionService::parse_detection_response(&llm_response)?;

    // Process completed steps
    let mut step_updates = Vec::new();
    for completed_step in &detection_result.completed_steps {
      // Update step status
      Self::update_step_status(db_state, completed_step.step_id, StepStatus::Completed)?;

      // Record progress
      Self::record_progress(
        db_state,
        task_id,
        Some(completed_step.step_id),
        Some(&completed_step.reasoning),
      )?;

      step_updates.push(StepUpdate {
        step_id: completed_step.step_id,
        status: StepStatus::Completed,
        reasoning: Some(completed_step.reasoning.clone()),
      });
    }

    // Check if task is complete
    let remaining_active_steps = Self::get_active_steps(db_state, task_id)?;
    let overall_status = if remaining_active_steps.is_empty() {
      Self::update_task_status(db_state, task_id, TaskStatus::Completed)?;
      TaskStatus::Completed
    } else {
      TaskStatus::Pending
    };
    Ok(TaskProgressUpdate {
      task_id,
      step_updates,
      overall_status,
    })
  }

  /// Update task status
  pub fn update_task_status(
    db_state: &DbState,
    task_id: i64,
    status: TaskStatus,
  ) -> Result<(), String> {
    let mut db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_mut()
      .ok_or("Database connection not available")?;

    conn
      .execute(
        "UPDATE tasks SET status = ?, updated_at = datetime('now') WHERE id = ?",
        params![status.as_str(), task_id],
      )
      .map_err(|e| format!("Failed to update task status: {}", e))?;

    Ok(())
  }

  /// Delete a task and all its steps
  pub fn delete_task(db_state: &DbState, task_id: i64) -> Result<(), String> {
    let mut db_guard = db_state
      .0
      .lock()
      .map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard
      .as_mut()
      .ok_or("Database connection not available")?;

    conn
      .execute("DELETE FROM tasks WHERE id = ?", params![task_id])
      .map_err(|e| format!("Failed to delete task: {}", e))?;

    Ok(())
  }

  /// Calculate progress percentage for a task
  fn calculate_progress_percentage(steps: &[TaskStep]) -> f64 {
    if steps.is_empty() {
      return 0.0;
    }

    let completed_count = steps
      .iter()
      .filter(|step| step.status == "completed")
      .count();

    (completed_count as f64 / steps.len() as f64) * 100.0
  }

  /// Parse datetime string to Utc DateTime - made public for external use
  pub fn parse_datetime(datetime_str: String) -> DateTime<Utc> {
    DateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
      .map(|dt| dt.with_timezone(&Utc))
      .unwrap_or_else(|_| Utc::now())
  }

  /// Complete a task and handle recurring tasks
  pub fn complete_task(db_state: &DbState, task_id: i64) -> Result<Option<TaskWithSteps>, String> {
    // Get the current task and its steps before starting the transaction
    let task = Self::get_task_by_id(db_state, task_id)?;
    let steps = Self::get_task_steps(db_state, task_id)?;
    let frequency = TaskFrequency::from_str(&task.frequency);
    let completion_time = Utc::now();

    let new_task_id = {
      let mut db_guard = db_state
        .0
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
      let conn = db_guard
        .as_mut()
        .ok_or("Database connection not available")?;

      // Start transaction
      let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Transaction error: {}", e))?;

      // Update current task as completed
      tx.execute(
                "UPDATE tasks SET status = 'completed', last_completed_at = ?, updated_at = datetime('now') WHERE id = ?",
                params![completion_time.format("%Y-%m-%d %H:%M:%S").to_string(), task_id],
            ).map_err(|e| format!("Failed to complete task: {}", e))?;

      // Mark all steps as completed
      tx.execute(
        "UPDATE task_steps SET status = 'completed', completed_at = ? WHERE task_id = ?",
        params![
          completion_time.format("%Y-%m-%d %H:%M:%S").to_string(),
          task_id
        ],
      )
      .map_err(|e| format!("Failed to complete task steps: {}", e))?;

      // If it's a recurring task, create a new instance
      let new_task_id = if frequency != TaskFrequency::OneTime {
        let new_first_scheduled = frequency
          .next_due_date(task.first_scheduled_at, Some(completion_time))
          .unwrap_or_else(|| completion_time + chrono::Duration::days(1)); // Fallback

        // Create new task for the next occurrence
        let new_task_id = tx.prepare("INSERT INTO tasks (name, description, category, priority, frequency, first_scheduled_at, status) VALUES (?, ?, ?, ?, ?, ?, 'pending')")
                    .and_then(|mut stmt| {
                        stmt.insert(params![
                            task.name,
                            task.description,
                            task.category,
                            task.priority,
                            task.frequency,
                            new_first_scheduled.format("%Y-%m-%d %H:%M:%S").to_string(),
                        ])
                    })
                    .map_err(|e| format!("Failed to create recurring task: {}", e))?;

        // Copy steps from original task (using the steps we retrieved earlier)
        for step in &steps {
          tx.execute(
                        "INSERT INTO task_steps (task_id, step_number, title, description, status) VALUES (?, ?, ?, ?, 'pending')",
                        params![new_task_id, step.step_number, &step.title, &step.description],
                    ).map_err(|e| format!("Failed to create recurring task step: {}", e))?;
        }

        Some(new_task_id)
      } else {
        None
      };

      // Commit transaction
      tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

      new_task_id
    }; // db_guard is dropped here

    // If we created a new recurring task, get it with its steps
    if let Some(task_id) = new_task_id {
      Ok(Some(Self::get_task_with_steps(db_state, task_id)?))
    } else {
      Ok(None)
    }
  }

  /// Get overdue tasks based on their frequency and first scheduled date
  pub fn get_overdue_tasks(db_state: &DbState) -> Result<Vec<TaskWithSteps>, String> {
    // Get all active tasks and filter overdue ones
    let all_tasks = Self::get_active_tasks(db_state)?;
    let overdue_tasks: Vec<TaskWithSteps> = all_tasks
      .into_iter()
      .filter(|task_with_steps| {
        let frequency = TaskFrequency::from_str(&task_with_steps.task.frequency);
        frequency.is_overdue(
          task_with_steps.task.first_scheduled_at,
          task_with_steps.task.last_completed_at,
        )
      })
      .collect();

    Ok(overdue_tasks)
  }

  /// Get tasks due today
  pub fn get_tasks_due_today(db_state: &DbState) -> Result<Vec<TaskWithSteps>, String> {
    // Get all active tasks and filter those due today
    let all_tasks = Self::get_active_tasks(db_state)?;
    let today_start = Utc::now()
      .date_naive()
      .and_hms_opt(0, 0, 0)
      .unwrap()
      .and_local_timezone(Utc)
      .unwrap();
    let today_end = Utc::now()
      .date_naive()
      .and_hms_opt(23, 59, 59)
      .unwrap()
      .and_local_timezone(Utc)
      .unwrap();

    let due_today_tasks: Vec<TaskWithSteps> = all_tasks
      .into_iter()
      .filter(|task_with_steps| {
        let frequency = TaskFrequency::from_str(&task_with_steps.task.frequency);
        if let Some(due_date) = frequency.next_due_date(
          task_with_steps.task.first_scheduled_at,
          task_with_steps.task.last_completed_at,
        ) {
          due_date >= today_start && due_date <= today_end
        } else {
          false
        }
      })
      .collect();

    Ok(due_today_tasks)
  }

  /// Get tasks by frequency type
  pub fn get_tasks_by_frequency(
    db_state: &DbState,
    frequency: TaskFrequency,
  ) -> Result<Vec<TaskWithSteps>, String> {
    let task_ids: Vec<i64> = {
      let db_guard = db_state
        .0
        .lock()
        .map_err(|e| format!("Database lock error: {}", e))?;
      let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

      let mut stmt = conn
        .prepare("SELECT id FROM tasks WHERE frequency = ? ORDER BY next_due_at ASC")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

      let task_ids: Vec<i64> = stmt
        .query_map([frequency.as_str()], |row| Ok(row.get(0)?))
        .map_err(|e| format!("Failed to query tasks by frequency: {}", e))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("Failed to collect task IDs: {}", e))?;

      task_ids
    }; // db_guard is dropped here

    let mut tasks = Vec::new();
    for task_id in task_ids {
      match Self::get_task_with_steps(db_state, task_id) {
        Ok(task_with_steps) => tasks.push(task_with_steps),
        Err(e) => log::error!("Failed to get task by frequency {}: {}", task_id, e),
      }
    }

    Ok(tasks)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_calculate_progress_percentage() {
    let steps = vec![
      TaskStep {
        id: 1,
        task_id: 1,
        step_number: 1,
        title: "Step 1".to_string(),
        description: "asdf".to_string(),
        status: "completed".to_string(),
        completed_at: None,
      },
      TaskStep {
        id: 2,
        task_id: 1,
        step_number: 2,
        title: "Step 2".to_string(),
        description: "asdf".to_string(),
        status: "pending".to_string(),
        completed_at: None,
      },
    ];

    let progress = TaskService::calculate_progress_percentage(&steps);
    assert_eq!(progress, 50.0);
  }
}
