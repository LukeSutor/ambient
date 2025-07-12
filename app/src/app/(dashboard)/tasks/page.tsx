"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Plus, AlertCircle, CheckCircle, Clock } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { TaskList } from "@/components/task-list";
import { TaskService } from "@/lib/task-service";
import { TaskWithSteps } from "@/types/tasks";
import { toast } from "sonner";

export default function TasksPage() {
  const router = useRouter();
  const [tasks, setTasks] = useState<TaskWithSteps[]>([]);
  const [loading, setLoading] = useState(true);
  const [deleting, setDeleting] = useState<number | null>(null);

  useEffect(() => {
    loadTasks();
  }, []);

  const loadTasks = async () => {
    try {
      setLoading(true);
      const allTasks = await TaskService.getAllTasks();
      setTasks(allTasks);
    } catch (error) {
      console.error("Failed to load tasks:", error);
      toast.error("Failed to load tasks");
    } finally {
      setLoading(false);
    }
  };

  const handleEdit = (taskWithSteps: TaskWithSteps) => {
    router.push(`/tasks/edit/${taskWithSteps.task.id}`);
  };

  const handleDelete = async (taskWithSteps: TaskWithSteps) => {
    if (!confirm(`Are you sure you want to delete "${taskWithSteps.task.name}"?`)) {
      return;
    }

    try {
      setDeleting(taskWithSteps.task.id);
      await TaskService.deleteTask(taskWithSteps.task.id);
      setTasks(tasks.filter(t => t.task.id !== taskWithSteps.task.id));
      toast.success("Task deleted successfully");
    } catch (error) {
      console.error("Failed to delete task:", error);
      toast.error("Failed to delete task");
    } finally {
      setDeleting(null);
    }
  };

  const handleComplete = async (taskWithSteps: TaskWithSteps) => {
    try {
      await TaskService.completeTask(taskWithSteps.task.id);
      await loadTasks(); // Reload to get updated task state
      toast.success("Task marked as complete!");
    } catch (error) {
      console.error("Failed to complete task:", error);
      toast.error("Failed to complete task");
    }
  };

  const handleNewTask = () => {
    router.push("/tasks/new");
  };

  // Calculate stats
  const dueToday = tasks.filter(taskWithSteps => 
    taskWithSteps.task.next_due_at && new Date(taskWithSteps.task.next_due_at).toDateString() === new Date().toDateString()
  ).length;
  
  const overdue = tasks.filter(taskWithSteps => 
    taskWithSteps.task.next_due_at && new Date(taskWithSteps.task.next_due_at) < new Date()
  ).length;

  if (loading) {
    return (
      <div className="container mx-auto py-8">
        <div className="flex items-center justify-center min-h-[400px]">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto py-8 space-y-8">
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Tasks</h1>
          <p className="text-muted-foreground">
            Manage your recurring and one-time tasks
          </p>
        </div>
        <Button onClick={handleNewTask} className="flex items-center gap-2">
          <Plus className="h-4 w-4" />
          New Task
        </Button>
      </div>

      {/* Stats Cards */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Tasks</CardTitle>
            <CheckCircle className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{tasks.length}</div>
            <p className="text-xs text-muted-foreground">
              All active tasks
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Due Today</CardTitle>
            <Clock className="h-4 w-4 text-orange-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-orange-600">{dueToday}</div>
            <p className="text-xs text-muted-foreground">
              Tasks to complete today
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Overdue</CardTitle>
            <AlertCircle className="h-4 w-4 text-red-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600">{overdue}</div>
            <p className="text-xs text-muted-foreground">
              Tasks past due date
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Task List */}
      <TaskList
        tasks={tasks}
        onEdit={handleEdit}
        onDelete={handleDelete}
        onComplete={handleComplete}
      />
    </div>
  );
}
