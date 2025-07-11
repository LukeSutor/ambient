export enum TaskFrequency {
  ONCE = "OneTime",
  DAILY = "Daily", 
  WEEKLY = "Weekly",
  MONTHLY = "Monthly",
  YEARLY = "Yearly"
}

export interface TaskStep {
  id: number;
  task_id: number;
  step_number: number;
  title: string;
  description?: string;
  status: string;
  completed_at?: string;
}

export interface Task {
  id: number;
  name: string;
  description?: string;
  category?: string;
  priority: number;
  frequency: string;
  last_completed_at?: string;
  next_due_at?: string;
  created_at: string;
  updated_at: string;
  status: string;
}

export interface TaskWithSteps {
  task: Task;
  steps: TaskStep[];
  progress_percentage: number;
}

export interface CreateTaskRequest {
  name: string;
  description?: string;
  category?: string;
  priority: number;
  frequency: TaskFrequency;
  steps: CreateTaskStepRequest[];
}

export interface CreateTaskStepRequest {
  title: string;
  description?: string;
}

export interface UpdateTaskRequest {
  id: number;
  name: string;
  description?: string;
  category?: string;
  priority: number;
  frequency: TaskFrequency;
  steps: CreateTaskStepRequest[];
}

export interface TaskProgress {
  id: number;
  task_id: number;
  step_id?: number;
  llm_confidence: number;
  evidence?: string;
  reasoning?: string;
  timestamp: string;
}

export interface TaskStatusCounts {
  total: number;
  due_today: number;
  overdue: number;
  completed_today: number;
}
