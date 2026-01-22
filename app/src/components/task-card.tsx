"use client";

import { TaskStatusBadge } from "@/components/task-status-badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Progress } from "@/components/ui/progress";
import {
  calculateNextDueDate,
  formatDueDate,
  getFrequencyDisplayName,
  getTaskCompletionProgress,
} from "@/lib/task-utils";
import type { TaskWithSteps } from "@/types/tasks";
import {
  CalendarDays,
  CheckCircle,
  Clock,
  Edit,
  MoreHorizontal,
  Trash2,
} from "lucide-react";
import { useState } from "react";

interface TaskCardProps {
  taskWithSteps: TaskWithSteps;
  onEdit: (taskWithSteps: TaskWithSteps) => void;
  onDelete: (taskWithSteps: TaskWithSteps) => void;
  onComplete: (taskWithSteps: TaskWithSteps) => void;
}

export function TaskCard({
  taskWithSteps,
  onEdit,
  onDelete,
  onComplete,
}: TaskCardProps) {
  const [isLoading, setIsLoading] = useState(false);
  const { completed, total, percentage } =
    getTaskCompletionProgress(taskWithSteps);
  const { task, steps } = taskWithSteps;

  // Calculate the next due date
  const nextDueDate = calculateNextDueDate(
    task.first_scheduled_at,
    task.frequency,
    task.last_completed_at,
  );

  const handleComplete = async () => {
    setIsLoading(true);
    try {
      await onComplete(taskWithSteps);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Card className="w-full hover:shadow-md transition-shadow">
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between">
          <div className="space-y-1 flex-1">
            <CardTitle className="text-lg">{task.name}</CardTitle>
            {task.description && (
              <CardDescription className="text-sm">
                {task.description}
              </CardDescription>
            )}
          </div>
          <div className="flex items-center gap-2">
            <TaskStatusBadge taskWithSteps={taskWithSteps} />
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                  <span className="sr-only">Open menu</span>
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onEdit(taskWithSteps)}>
                  <Edit className="mr-2 h-4 w-4" />
                  Edit
                </DropdownMenuItem>
                <DropdownMenuItem onClick={handleComplete} disabled={isLoading}>
                  <CheckCircle className="mr-2 h-4 w-4" />
                  Mark Complete
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => onDelete(taskWithSteps)}
                  className="text-red-600"
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Progress */}
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span>Progress</span>
            <span>
              {completed}/{total} steps
            </span>
          </div>
          <Progress value={percentage} className="h-2" />
        </div>

        {/* Task Info */}
        <div className="flex items-center justify-between text-sm text-muted-foreground">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              <span>{getFrequencyDisplayName(task.frequency)}</span>
            </div>
            {nextDueDate && (
              <div className="flex items-center gap-1">
                <CalendarDays className="h-3 w-3" />
                <span>{formatDueDate(nextDueDate.toISOString())}</span>
              </div>
            )}
          </div>
        </div>

        {/* Steps preview */}
        {steps.length > 0 && (
          <div className="space-y-1">
            <div className="text-sm font-medium">Steps:</div>
            <div className="space-y-1 max-h-20 overflow-y-auto">
              {steps.slice(0, 3).map((step) => (
                <div key={step.id} className="flex items-center gap-2 text-sm">
                  <div
                    className={`h-2 w-2 rounded-full ${step.status === "Completed" ? "bg-green-500" : "bg-gray-300"}`}
                  />
                  <span
                    className={
                      step.status === "Completed"
                        ? "line-through text-muted-foreground"
                        : ""
                    }
                  >
                    {step.title}
                  </span>
                </div>
              ))}
              {steps.length > 3 && (
                <div className="text-xs text-muted-foreground">
                  +{steps.length - 3} more steps
                </div>
              )}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
