"use client";

import { useState } from "react";
import { TaskCard } from "@/components/task-card";
import { TaskWithSteps, TaskFrequency } from "@/types/tasks";
import { sortTasksByPriority, getTaskStatus } from "@/lib/task-utils";
import { 
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Filter, SortDesc } from "lucide-react";

interface TaskListProps {
  tasks: TaskWithSteps[];
  onEdit: (taskWithSteps: TaskWithSteps) => void;
  onDelete: (taskWithSteps: TaskWithSteps) => void;
  onComplete: (taskWithSteps: TaskWithSteps) => void;
}

type SortOption = "priority" | "name" | "created" | "due";
type FilterOption = "all" | "due-today" | "overdue" | "upcoming" | "once" | "daily" | "weekly" | "monthly" | "yearly";

export function TaskList({ tasks, onEdit, onDelete, onComplete }: TaskListProps) {
  const [sortBy, setSortBy] = useState<SortOption>("priority");
  const [filterBy, setFilterBy] = useState<FilterOption>("all");

  const filteredTasks = tasks.filter((taskWithSteps) => {
    if (filterBy === "all") return true;
    
    const status = getTaskStatus(taskWithSteps);
    if (filterBy === "due-today") {
      return status === "due-today";
    }
    
    if (filterBy === "overdue") {
      return status === "overdue";
    }
    
    if (filterBy === "upcoming") {
      return status === "upcoming";
    }
    
    return taskWithSteps.task.frequency === filterBy;
  });

  const sortedTasks = [...filteredTasks].sort((a, b) => {
    switch (sortBy) {
      case "priority":
        return sortTasksByPriority([a, b]).indexOf(a) - sortTasksByPriority([a, b]).indexOf(b);
      case "name":
        return a.task.name.localeCompare(b.task.name);
      case "created":
        return new Date(b.task.created_at).getTime() - new Date(a.task.created_at).getTime();
      case "due":
        if (!a.task.next_due_at && !b.task.next_due_at) return 0;
        if (!a.task.next_due_at) return 1;
        if (!b.task.next_due_at) return -1;
        return new Date(a.task.next_due_at).getTime() - new Date(b.task.next_due_at).getTime();
      default:
        return 0;
    }
  });

  if (tasks.length === 0) {
    return (
      <div className="text-center py-12">
        <div className="text-muted-foreground">
          <h3 className="text-lg font-medium mb-2">No tasks yet</h3>
          <p>Create your first task to get started!</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Filters and Sort */}
      <div className="flex flex-col sm:flex-row gap-4">
        <div className="flex items-center gap-2">
          <Filter className="h-4 w-4" />
          <Select value={filterBy} onValueChange={(value: string) => setFilterBy(value as FilterOption)}>
            <SelectTrigger className="w-[180px]">
              <SelectValue placeholder="Filter by" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Tasks</SelectItem>
              <SelectItem value="once">One-time</SelectItem>
              <SelectItem value="daily">Daily</SelectItem>
              <SelectItem value="weekly">Weekly</SelectItem>
              <SelectItem value="monthly">Monthly</SelectItem>
              <SelectItem value="yearly">Yearly</SelectItem>
              <SelectItem value="due-today">Due Today</SelectItem>
              <SelectItem value="overdue">Overdue</SelectItem>
              <SelectItem value="upcoming">Upcoming</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center gap-2">
          <SortDesc className="h-4 w-4" />
          <Select value={sortBy} onValueChange={(value: string) => setSortBy(value as SortOption)}>
            <SelectTrigger className="w-[180px]">
              <SelectValue placeholder="Sort by" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="priority">Priority</SelectItem>
              <SelectItem value="name">Name</SelectItem>
              <SelectItem value="created">Created Date</SelectItem>
              <SelectItem value="due">Due Date</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Task Count */}
      <div className="text-sm text-muted-foreground">
        Showing {sortedTasks.length} of {tasks.length} tasks
      </div>

      {/* Task Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {sortedTasks.map((taskWithSteps) => (
          <TaskCard
            key={taskWithSteps.task.id}
            taskWithSteps={taskWithSteps}
            onEdit={onEdit}
            onDelete={onDelete}
            onComplete={onComplete}
          />
        ))}
      </div>
    </div>
  );
}
