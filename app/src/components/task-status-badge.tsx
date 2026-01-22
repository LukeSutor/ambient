"use client";

import { Badge } from "@/components/ui/badge";
import {
  getTaskStatus,
  getTaskStatusBadgeColor,
  getTaskStatusText,
} from "@/lib/task-utils";
import type { TaskWithSteps } from "@/types/tasks";

interface TaskStatusBadgeProps {
  taskWithSteps: TaskWithSteps;
  className?: string;
}

export function TaskStatusBadge({
  taskWithSteps,
  className,
}: TaskStatusBadgeProps) {
  const status = getTaskStatus(taskWithSteps);
  const colorClass = getTaskStatusBadgeColor(status);
  const text = getTaskStatusText(status);

  return (
    <Badge variant="outline" className={`${colorClass} border-0 ${className}`}>
      {text}
    </Badge>
  );
}
