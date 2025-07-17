"use client";

import { invoke } from "@tauri-apps/api/core";
import { 
  TaskWithSteps, 
  CreateTaskRequest, 
  UpdateTaskRequest, 
  TaskProgress,
  TaskStatusCounts,
  TaskFrequency 
} from "@/types/tasks";

export class TaskService {
  static async getAllTasks(): Promise<TaskWithSteps[]> {
    try {
      return await invoke<TaskWithSteps[]>("get_active_tasks");
    } catch (error) {
      console.error("Failed to get all tasks:", error);
      throw error;
    }
  }

  static async getTask(taskId: bigint): Promise<TaskWithSteps> {
    try {
      return await invoke<TaskWithSteps>("get_task", { taskId });
    } catch (error) {
      console.error("Failed to get task:", error);
      throw error;
    }
  }

  static async createTask(request: CreateTaskRequest): Promise<TaskWithSteps> {
    try {
      return await invoke<TaskWithSteps>("create_task", { request });
    } catch (error) {
      console.error("Failed to create task:", error);
      throw error;
    }
  }

  static async updateTask(request: UpdateTaskRequest): Promise<TaskWithSteps> {
    try {
      // For now, we'll just delete and recreate - can be improved later
      await this.deleteTask(request.id);
      const createRequest: CreateTaskRequest = {
        name: request.name,
        description: request.description,
        category: request.category,
        priority: request.priority,
        frequency: request.frequency,
        steps: request.steps
      };
      return await this.createTask(createRequest);
    } catch (error) {
      console.error("Failed to update task:", error);
      throw error;
    }
  }

  static async deleteTask(taskId: bigint): Promise<void> {
    try {
      await invoke<void>("delete_task", { taskId });
    } catch (error) {
      console.error("Failed to delete task:", error);
      throw error;
    }
  }

  static async completeTask(taskId: bigint): Promise<TaskWithSteps | null> {
    try {
      return await invoke<TaskWithSteps | null>("complete_task", { taskId });
    } catch (error) {
      console.error("Failed to complete task:", error);
      throw error;
    }
  }

  static async updateTaskStatus(taskId: bigint, status: string): Promise<void> {
    try {
      await invoke<void>("update_task_status", { taskId, status });
    } catch (error) {
      console.error("Failed to update task status:", error);
      throw error;
    }
  }

  static async updateStepStatus(stepId: bigint, status: string): Promise<void> {
    try {
      await invoke<void>("update_step_status", { stepId, status });
    } catch (error) {
      console.error("Failed to update step status:", error);
      throw error;
    }
  }

  static async getTasksDueToday(): Promise<TaskWithSteps[]> {
    try {
      return await invoke<TaskWithSteps[]>("get_tasks_due_today");
    } catch (error) {
      console.error("Failed to get tasks due today:", error);
      throw error;
    }
  }

  static async getOverdueTasks(): Promise<TaskWithSteps[]> {
    try {
      return await invoke<TaskWithSteps[]>("get_overdue_tasks");
    } catch (error) {
      console.error("Failed to get overdue tasks:", error);
      throw error;
    }
  }

  static async getTasksByFrequency(frequency: string): Promise<TaskWithSteps[]> {
    try {
      return await invoke<TaskWithSteps[]>("get_tasks_by_frequency", { frequency });
    } catch (error) {
      console.error("Failed to get tasks by frequency:", error);
      throw error;
    }
  }

  static async getTaskProgressHistory(taskId: number): Promise<TaskProgress[]> {
    try {
      return await invoke<TaskProgress[]>("get_task_progress_history", { taskId });
    } catch (error) {
      console.error("Failed to get task progress history:", error);
      throw error;
    }
  }
}
