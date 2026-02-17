"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import type {
  AutomationRun,
  AutomationTask,
  CreateAutomationParams,
} from "@/types/automations";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Bot,
  Calendar,
  Clock,
  Eye,
  Loader2,
  MonitorPlay,
  Pencil,
  Play,
  Plus,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

// ── Helpers ──────────────────────────────────────────────────────────

function formatDate(d: string | null): string {
  if (!d) return "Never";
  const date = new Date(d);
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function scheduleLabel(task: AutomationTask): string {
  if (task.task_type === "semantic") {
    return task.trigger_type ?? "Trigger";
  }
  switch (task.schedule_type) {
    case "interval":
      return `Every ${task.schedule_value ?? "?"} min`;
    case "daily":
      return `Daily at ${task.schedule_value ?? "?"}`;
    case "weekly":
      return `Weekly: ${task.schedule_value ?? "?"}`;
    case "once":
      return "One-time";
    default:
      return "No schedule";
  }
}

function taskTypeIcon(task: AutomationTask) {
  if (task.task_type === "semantic") {
    return <Eye className="h-4 w-4" />;
  }
  switch (task.schedule_type) {
    case "interval":
      return <Clock className="h-4 w-4" />;
    case "daily":
    case "weekly":
      return <Calendar className="h-4 w-4" />;
    default:
      return <Bot className="h-4 w-4" />;
  }
}

// ── Main Page ────────────────────────────────────────────────────────

export default function AutomationsPage() {
  const [tasks, setTasks] = useState<AutomationTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editTask, setEditTask] = useState<AutomationTask | null>(null);
  const [runHistoryTask, setRunHistoryTask] = useState<AutomationTask | null>(
    null,
  );

  const fetchTasks = useCallback(async () => {
    try {
      const result = await invoke<AutomationTask[]>("get_automation_tasks");
      setTasks(result);
    } catch (e) {
      console.error("Failed to load automations:", e);
      toast.error("Failed to load automations");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTasks();
  }, [fetchTasks]);

  // Listen for events
  useEffect(() => {
    const listeners = [
      listen("automation_task_created", () => fetchTasks()),
      listen("automation_task_updated", () => fetchTasks()),
      listen("automation_task_deleted", () => fetchTasks()),
      listen("automation_run_completed", () => fetchTasks()),
    ];

    return () => {
      for (const listener of listeners) {
        listener.then((unlisten) => unlisten());
      }
    };
  }, [fetchTasks]);

  const handleToggle = async (task: AutomationTask, enabled: boolean) => {
    try {
      await invoke("toggle_automation_task", {
        taskId: task.id,
        enabled,
      });
      toast.success(`${task.name} ${enabled ? "enabled" : "disabled"}`);
      fetchTasks();
    } catch (e) {
      toast.error(`Failed to toggle: ${e}`);
    }
  };

  const handleDelete = async (task: AutomationTask) => {
    try {
      await invoke("delete_automation_task", { taskId: task.id });
      toast.success(`${task.name} deleted`);
      fetchTasks();
    } catch (e) {
      toast.error(`Failed to delete: ${e}`);
    }
  };

  const handleRunNow = async (task: AutomationTask) => {
    try {
      toast.info(`Running "${task.name}"...`);
      await invoke<AutomationRun>("run_automation_task", { taskId: task.id });
      toast.success(`"${task.name}" completed`);
      fetchTasks();
    } catch (e) {
      toast.error(`Run failed: ${e}`);
    }
  };

  const handleEdit = (task: AutomationTask) => {
    setEditTask(task);
    setDialogOpen(true);
  };

  const handleCreate = () => {
    setEditTask(null);
    setDialogOpen(true);
  };

  const handleDialogClose = () => {
    setDialogOpen(false);
    setEditTask(null);
    fetchTasks();
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const scheduledTasks = tasks.filter((t) => t.task_type === "scheduled");
  const semanticTasks = tasks.filter((t) => t.task_type === "semantic");

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full max-w-6xl mx-auto">
      <div className="flex items-center justify-between w-full mb-6">
        <div>
          <h1 className="text-3xl font-bold font-sora">Automations</h1>
          <p className="text-muted-foreground mt-1">
            Create and manage background tasks that run on a schedule or
            trigger.
          </p>
        </div>
        <Button onClick={handleCreate}>
          <Plus className="h-4 w-4 mr-2" />
          New Automation
        </Button>
      </div>

      {tasks.length === 0 ? (
        <EmptyState onCreate={handleCreate} />
      ) : (
        <div className="w-full space-y-8">
          {scheduledTasks.length > 0 && (
            <TaskSection
              title="Scheduled Tasks"
              icon={<Clock className="h-5 w-5" />}
              tasks={scheduledTasks}
              onToggle={handleToggle}
              onDelete={handleDelete}
              onRunNow={handleRunNow}
              onEdit={handleEdit}
              onViewRuns={(t) => setRunHistoryTask(t)}
            />
          )}

          {semanticTasks.length > 0 && (
            <TaskSection
              title="Trigger-Based Tasks"
              icon={<Eye className="h-5 w-5" />}
              tasks={semanticTasks}
              onToggle={handleToggle}
              onDelete={handleDelete}
              onRunNow={handleRunNow}
              onEdit={handleEdit}
              onViewRuns={(t) => setRunHistoryTask(t)}
            />
          )}
        </div>
      )}

      {/* Create/Edit Dialog */}
      <AutomationDialog
        open={dialogOpen}
        onClose={handleDialogClose}
        editTask={editTask}
      />

      {/* Run History Dialog */}
      {runHistoryTask && (
        <RunHistoryDialog
          task={runHistoryTask}
          onClose={() => setRunHistoryTask(null)}
        />
      )}
    </div>
  );
}

// ── Empty State ──────────────────────────────────────────────────────

function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <Card className="w-full max-w-md mx-auto">
      <CardContent className="flex flex-col items-center py-12 text-center">
        <Bot className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">No automations yet</h3>
        <p className="text-sm text-muted-foreground mb-6">
          Create your first automation to run tasks on a schedule or in response
          to screen events.
        </p>
        <Button onClick={onCreate}>
          <Plus className="h-4 w-4 mr-2" />
          Create Automation
        </Button>
      </CardContent>
    </Card>
  );
}

// ── Task Section ─────────────────────────────────────────────────────

function TaskSection({
  title,
  icon,
  tasks,
  onToggle,
  onDelete,
  onRunNow,
  onEdit,
  onViewRuns,
}: {
  title: string;
  icon: React.ReactNode;
  tasks: AutomationTask[];
  onToggle: (task: AutomationTask, enabled: boolean) => void;
  onDelete: (task: AutomationTask) => void;
  onRunNow: (task: AutomationTask) => void;
  onEdit: (task: AutomationTask) => void;
  onViewRuns: (task: AutomationTask) => void;
}) {
  return (
    <div>
      <div className="flex items-center gap-2 mb-3">
        {icon}
        <h2 className="text-lg font-semibold">{title}</h2>
        <Badge variant="secondary" className="ml-1">
          {tasks.length}
        </Badge>
      </div>
      <div className="grid gap-3">
        {tasks.map((task) => (
          <TaskCard
            key={task.id}
            task={task}
            onToggle={onToggle}
            onDelete={onDelete}
            onRunNow={onRunNow}
            onEdit={onEdit}
            onViewRuns={onViewRuns}
          />
        ))}
      </div>
    </div>
  );
}

// ── Task Card ────────────────────────────────────────────────────────

function TaskCard({
  task,
  onToggle,
  onDelete,
  onRunNow,
  onEdit,
  onViewRuns,
}: {
  task: AutomationTask;
  onToggle: (task: AutomationTask, enabled: boolean) => void;
  onDelete: (task: AutomationTask) => void;
  onRunNow: (task: AutomationTask) => void;
  onEdit: (task: AutomationTask) => void;
  onViewRuns: (task: AutomationTask) => void;
}) {
  return (
    <Card className="hover:shadow-sm transition-shadow">
      <CardContent className="flex items-center justify-between py-4 px-5">
        <div className="flex items-center gap-4 flex-1 min-w-0">
          <Switch
            checked={task.is_enabled}
            onCheckedChange={(checked) => onToggle(task, checked)}
          />

          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-medium truncate">{task.name}</span>
              {task.is_system && (
                <Badge variant="outline" className="text-xs">
                  System
                </Badge>
              )}
            </div>
            {task.description && (
              <p className="text-sm text-muted-foreground truncate mt-0.5">
                {task.description}
              </p>
            )}
          </div>

          <div className="flex items-center gap-2 text-sm text-muted-foreground shrink-0">
            {taskTypeIcon(task)}
            <span>{scheduleLabel(task)}</span>
          </div>

          <div className="text-xs text-muted-foreground shrink-0 w-28 text-right">
            Last run: {formatDate(task.last_run_at)}
          </div>
        </div>

        <div className="flex items-center gap-1 ml-4 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => onRunNow(task)}
            title="Run now"
          >
            <Play className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => onViewRuns(task)}
            title="View run history"
          >
            <MonitorPlay className="h-4 w-4" />
          </Button>
          {!task.is_system && (
            <>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={() => onEdit(task)}
                title="Edit"
              >
                <Pencil className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-destructive"
                onClick={() => onDelete(task)}
                title="Delete"
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            </>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ── Create/Edit Dialog ───────────────────────────────────────────────

function AutomationDialog({
  open,
  onClose,
  editTask,
}: {
  open: boolean;
  onClose: () => void;
  editTask: AutomationTask | null;
}) {
  const isEdit = !!editTask;
  const [saving, setSaving] = useState(false);

  // Form state
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [taskType, setTaskType] = useState<string>("scheduled");
  const [promptTemplate, setPromptTemplate] = useState("");
  const [scheduleType, setScheduleType] = useState<string>("interval");
  const [scheduleValue, setScheduleValue] = useState("");
  const [triggerType, setTriggerType] = useState<string>("screen_content");
  const [triggerConfig, setTriggerConfig] = useState("");
  const [maxIterations, setMaxIterations] = useState("10");
  const [timeoutSeconds, setTimeoutSeconds] = useState("120");

  // Populate form when editing
  useEffect(() => {
    if (editTask) {
      setName(editTask.name);
      setDescription(editTask.description);
      setTaskType(editTask.task_type);
      setPromptTemplate(editTask.prompt_template);
      setScheduleType(editTask.schedule_type ?? "interval");
      setScheduleValue(editTask.schedule_value ?? "");
      setTriggerType(editTask.trigger_type ?? "screen_content");
      setTriggerConfig(editTask.trigger_config ?? "");
      setMaxIterations(String(editTask.max_iterations));
      setTimeoutSeconds(String(editTask.timeout_seconds));
    } else {
      setName("");
      setDescription("");
      setTaskType("scheduled");
      setPromptTemplate("");
      setScheduleType("interval");
      setScheduleValue("");
      setTriggerType("screen_content");
      setTriggerConfig("");
      setMaxIterations("10");
      setTimeoutSeconds("120");
    }
  }, [editTask, open]);

  const handleSave = async () => {
    if (!name.trim() || !promptTemplate.trim()) {
      toast.error("Name and prompt are required");
      return;
    }

    setSaving(true);
    try {
      if (isEdit && editTask) {
        await invoke("update_automation_task", {
          params: {
            id: editTask.id,
            name,
            description: description || null,
            prompt_template: promptTemplate,
            schedule_type: taskType === "scheduled" ? scheduleType : null,
            schedule_value:
              taskType === "scheduled" ? scheduleValue || null : null,
            trigger_type: taskType === "semantic" ? triggerType : null,
            trigger_config:
              taskType === "semantic" ? triggerConfig || null : null,
            max_iterations: Number.parseInt(maxIterations) || null,
            timeout_seconds: Number.parseInt(timeoutSeconds) || null,
          },
        });
        toast.success("Automation updated");
      } else {
        const params: CreateAutomationParams = {
          name,
          description: description || null,
          task_type: taskType,
          prompt_template: promptTemplate,
          model_id: null,
          disabled_skills: null,
          notify_on_complete: true,
          notify_on_error: true,
          max_iterations: Number.parseInt(maxIterations) || 10,
          timeout_seconds: Number.parseInt(timeoutSeconds) || 120,
          schedule_type: taskType === "scheduled" ? scheduleType : null,
          schedule_value:
            taskType === "scheduled" ? scheduleValue || null : null,
          schedule_timezone: null,
          trigger_type: taskType === "semantic" ? triggerType : null,
          trigger_config:
            taskType === "semantic" ? triggerConfig || null : null,
        };
        await invoke("create_automation_task", { params });
        toast.success("Automation created");
      }
      onClose();
    } catch (e) {
      toast.error(`Failed to save: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-lg max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {isEdit ? "Edit Automation" : "Create Automation"}
          </DialogTitle>
          <DialogDescription>
            {isEdit
              ? "Modify this automation task."
              : "Set up a new background task."}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 mt-2">
          {/* Name */}
          <div className="space-y-1.5">
            <Label htmlFor="name">Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Daily Summary"
            />
          </div>

          {/* Description */}
          <div className="space-y-1.5">
            <Label htmlFor="description">Description</Label>
            <Input
              id="description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="A brief description of what this does"
            />
          </div>

          {/* Task Type */}
          <div className="space-y-1.5">
            <Label>Type</Label>
            <Select value={taskType} onValueChange={setTaskType}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="scheduled">
                  Scheduled (time-based)
                </SelectItem>
                <SelectItem value="semantic">
                  Trigger-based (event-driven)
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Schedule fields */}
          {taskType === "scheduled" && (
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label>Schedule Type</Label>
                <Select value={scheduleType} onValueChange={setScheduleType}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="interval">Every N minutes</SelectItem>
                    <SelectItem value="daily">Daily at time</SelectItem>
                    <SelectItem value="weekly">Weekly</SelectItem>
                    <SelectItem value="once">One-time</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>
                  {scheduleType === "interval"
                    ? "Minutes"
                    : scheduleType === "daily"
                      ? "Time (HH:MM)"
                      : scheduleType === "weekly"
                        ? "Day,Time"
                        : "DateTime (ISO)"}
                </Label>
                <Input
                  value={scheduleValue}
                  onChange={(e) => setScheduleValue(e.target.value)}
                  placeholder={
                    scheduleType === "interval"
                      ? "30"
                      : scheduleType === "daily"
                        ? "09:00"
                        : scheduleType === "weekly"
                          ? "monday,09:00"
                          : "2025-01-01T00:00:00Z"
                  }
                />
              </div>
            </div>
          )}

          {/* Trigger fields */}
          {taskType === "semantic" && (
            <div className="space-y-3">
              <div className="space-y-1.5">
                <Label>Trigger Type</Label>
                <Select value={triggerType} onValueChange={setTriggerType}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="screen_content">
                      Screen Content
                    </SelectItem>
                    <SelectItem value="url_visit">URL Visit</SelectItem>
                    <SelectItem value="app_focus">App Focus</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>Trigger Config (JSON)</Label>
                <Textarea
                  value={triggerConfig}
                  onChange={(e) => setTriggerConfig(e.target.value)}
                  placeholder='{"keywords": ["error", "alert"]}'
                  rows={3}
                  className="font-mono text-sm"
                />
              </div>
            </div>
          )}

          {/* Prompt */}
          <div className="space-y-1.5">
            <Label htmlFor="prompt">Prompt</Label>
            <Textarea
              id="prompt"
              value={promptTemplate}
              onChange={(e) => setPromptTemplate(e.target.value)}
              placeholder="What should this automation do when it runs?"
              rows={4}
            />
          </div>

          {/* Advanced */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label>Max Iterations</Label>
              <Input
                type="number"
                value={maxIterations}
                onChange={(e) => setMaxIterations(e.target.value)}
                min={1}
                max={50}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Timeout (seconds)</Label>
              <Input
                type="number"
                value={timeoutSeconds}
                onChange={(e) => setTimeoutSeconds(e.target.value)}
                min={10}
                max={600}
              />
            </div>
          </div>

          {/* Save button */}
          <div className="flex justify-end pt-2">
            <Button onClick={handleSave} disabled={saving}>
              {saving && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              {isEdit ? "Save Changes" : "Create Automation"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ── Run History Dialog ───────────────────────────────────────────────

function RunHistoryDialog({
  task,
  onClose,
}: {
  task: AutomationTask;
  onClose: () => void;
}) {
  const [runs, setRuns] = useState<AutomationRun[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchRuns = async () => {
      try {
        const result = await invoke<AutomationRun[]>("get_automation_runs", {
          taskId: task.id,
          limit: 20,
          offset: 0,
        });
        setRuns(result);
      } catch (e) {
        console.error("Failed to fetch runs:", e);
      } finally {
        setLoading(false);
      }
    };
    fetchRuns();
  }, [task.id]);

  return (
    <Dialog open onOpenChange={() => onClose()}>
      <DialogContent className="max-w-xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Run History — {task.name}</DialogTitle>
          <DialogDescription>
            Recent execution history for this automation.
          </DialogDescription>
        </DialogHeader>

        {loading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-5 w-5 animate-spin" />
          </div>
        ) : runs.length === 0 ? (
          <p className="text-sm text-muted-foreground py-6 text-center">
            No runs yet.
          </p>
        ) : (
          <div className="space-y-2 mt-2">
            {runs.map((run) => (
              <Card key={run.id}>
                <CardContent className="py-3 px-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Badge
                        variant={
                          run.status === "completed"
                            ? "default"
                            : run.status === "failed"
                              ? "destructive"
                              : "secondary"
                        }
                      >
                        {run.status}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {formatDate(run.started_at)}
                      </span>
                    </div>
                    {run.credits_used > 0 && (
                      <span className="text-xs text-muted-foreground">
                        {run.credits_used} credits
                      </span>
                    )}
                  </div>
                  {run.result_text && (
                    <p className="text-sm mt-2 text-muted-foreground line-clamp-3">
                      {run.result_text}
                    </p>
                  )}
                  {run.error_message && (
                    <p className="text-sm mt-2 text-destructive line-clamp-3">
                      {run.error_message}
                    </p>
                  )}
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
