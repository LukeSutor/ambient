import { TaskWithSteps, TaskFrequency } from "@/types/tasks";

export function getTaskStatus(taskWithSteps: TaskWithSteps): "due-today" | "overdue" | "upcoming" | "completed" {
  const task = taskWithSteps.task;
  
  if (!task.next_due_at) {
    return "completed";
  }

  const dueDate = new Date(task.next_due_at);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  
  const dueDateOnly = new Date(dueDate);
  dueDateOnly.setHours(0, 0, 0, 0);
  
  if (dueDateOnly.getTime() === today.getTime()) {
    return "due-today";
  }
  
  if (dueDateOnly < today) {
    return "overdue";
  }
  
  return "upcoming";
}

export function getTaskStatusBadgeColor(status: "due-today" | "overdue" | "upcoming" | "completed"): string {
  switch (status) {
    case "overdue":
      return "bg-red-500 text-white";
    case "due-today":
      return "bg-orange-500 text-white";
    case "upcoming":
      return "bg-blue-500 text-white";
    case "completed":
      return "bg-green-500 text-white";
    default:
      return "bg-gray-500 text-white";
  }
}

export function getTaskStatusText(status: "due-today" | "overdue" | "upcoming" | "completed"): string {
  switch (status) {
    case "overdue":
      return "Overdue";
    case "due-today":
      return "Due Today";
    case "upcoming":
      return "Upcoming";
    case "completed":
      return "Completed";
    default:
      return "Unknown";
  }
}

export function formatDueDate(dateString: string): string {
  try {
    const date = new Date(dateString);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    
    const dateOnly = new Date(date);
    dateOnly.setHours(0, 0, 0, 0);
    
    const diffTime = dateOnly.getTime() - today.getTime();
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
    
    if (diffDays === 0) {
      return "Today";
    }
    
    if (diffDays < 0) {
      const daysPast = Math.abs(diffDays);
      return `${daysPast} day${daysPast !== 1 ? 's' : ''} ago`;
    }
    
    if (diffDays === 1) {
      return "Tomorrow";
    }
    
    if (diffDays <= 7) {
      return `In ${diffDays} day${diffDays !== 1 ? 's' : ''}`;
    }
    
    return date.toLocaleDateString('en-US', { 
      month: 'short', 
      day: 'numeric', 
      year: 'numeric' 
    });
  } catch (error) {
    return "Invalid date";
  }
}

export function getFrequencyDisplayName(frequency: string): string {
  switch (frequency) {
    case "OneTime":
      return "One-time";
    case "Daily":
      return "Daily";
    case "Weekly":
      return "Weekly";
    case "Monthly":
      return "Monthly";
    case "Yearly":
      return "Yearly";
    default:
      return frequency;
  }
}

export function sortTasksByPriority(tasks: TaskWithSteps[]): TaskWithSteps[] {
  return tasks.sort((a, b) => {
    const statusA = getTaskStatus(a);
    const statusB = getTaskStatus(b);
    
    // Priority order: overdue > due-today > upcoming > completed
    const statusPriority = {
      "overdue": 0,
      "due-today": 1,
      "upcoming": 2,
      "completed": 3
    };
    
    const priorityA = statusPriority[statusA];
    const priorityB = statusPriority[statusB];
    
    if (priorityA !== priorityB) {
      return priorityA - priorityB;
    }
    
    // If same status, sort by due date
    if (a.task.next_due_at && b.task.next_due_at) {
      return new Date(a.task.next_due_at).getTime() - new Date(b.task.next_due_at).getTime();
    }
    
    // If no due dates, sort by creation date (newest first)
    return new Date(b.task.created_at).getTime() - new Date(a.task.created_at).getTime();
  });
}

export function getTaskCompletionProgress(taskWithSteps: TaskWithSteps): { completed: number; total: number; percentage: number } {
  const total = taskWithSteps.steps.length;
  const completed = taskWithSteps.steps.filter(step => step.status === "Completed").length;
  const percentage = total > 0 ? Math.round((completed / total) * 100) : 0;
  
  return { completed, total, percentage };
}
