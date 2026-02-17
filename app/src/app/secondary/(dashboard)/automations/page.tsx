"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
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
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { useModelAccess } from "@/lib/model-access/useModelAccess";
import type {
  AutomationRun,
  AutomationTask,
  CreateAutomationParams,
} from "@/types/automations";
import type { ModelEntry } from "@/types/models";
import type { SkillSummary } from "@/types/skills";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Bot,
  Calendar,
  ChevronDown,
  Clock,
  Eye,
  Loader2,
  MonitorPlay,
  Pencil,
  Play,
  Plus,
  Trash2,
  X,
} from "lucide-react";
import {
  type KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { toast } from "sonner";

// ── Time Helpers ─────────────────────────────────────────────────────

/** Parse flexible time input ("5pm", "5:00 PM", "17:00") → "HH:MM" 24h. */
function parseTimeInput(input: string): string | null {
  const s = input.trim().toLowerCase().replace(/\s+/g, "");
  // HH:MM AM/PM
  let m = s.match(/^(\d{1,2}):(\d{2})(am|pm)$/);
  if (m) {
    let h = Number.parseInt(m[1]);
    const min = Number.parseInt(m[2]);
    if (m[3] === "pm" && h < 12) h += 12;
    if (m[3] === "am" && h === 12) h = 0;
    if (h <= 23 && min <= 59)
      return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
  }
  // H AM/PM (e.g. "5pm")
  m = s.match(/^(\d{1,2})(am|pm)$/);
  if (m) {
    let h = Number.parseInt(m[1]);
    if (m[2] === "pm" && h < 12) h += 12;
    if (m[2] === "am" && h === 12) h = 0;
    if (h <= 23) return `${String(h).padStart(2, "0")}:00`;
  }
  // HH:MM 24h
  m = s.match(/^(\d{1,2}):(\d{2})$/);
  if (m) {
    const h = Number.parseInt(m[1]);
    const min = Number.parseInt(m[2]);
    if (h <= 23 && min <= 59)
      return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
  }
  return null;
}

/** Format "HH:MM" (24h) → "5:00 PM" (12h). */
function formatTime12h(time24: string): string {
  const [hStr, mStr] = time24.split(":");
  let h = Number.parseInt(hStr ?? "0");
  const min = mStr ?? "00";
  const period = h >= 12 ? "PM" : "AM";
  if (h > 12) h -= 12;
  if (h === 0) h = 12;
  return `${h}:${min} ${period}`;
}

/** Format an ISO timestamp for display in 12h local time. */
function formatDate(d: string | null): string {
  if (!d) return "Never";
  const date = new Date(d);
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}

/** Smart formatting for next run time. */
function formatNextRun(d: string | null): string {
  if (!d) return "—";
  const now = new Date();
  const next = new Date(d);
  const diffMs = next.getTime() - now.getTime();
  if (diffMs < 0) return "Overdue";

  const diffHours = diffMs / (1000 * 60 * 60);
  const diffDays = diffMs / (1000 * 60 * 60 * 24);

  const timeStr = next.toLocaleTimeString(undefined, {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });

  // Same day
  if (next.toDateString() === now.toDateString()) {
    return `Today at ${timeStr}`;
  }

  // Within 48 hours (tomorrow)
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (next.toDateString() === tomorrow.toDateString()) {
    return `Tomorrow at ${timeStr}`;
  }

  // Within 7 days – show weekday name
  if (diffDays <= 7) {
    const dayName = next.toLocaleDateString(undefined, { weekday: "long" });
    return `${dayName} at ${timeStr}`;
  }

  // More than 7 days – show date
  const dateStr = next.toLocaleDateString(undefined, {
    month: "numeric",
    day: "numeric",
  });
  return `${dateStr} at ${timeStr}`;
}

const ALL_DAYS = [
  { key: "mon", label: "Mon" },
  { key: "tue", label: "Tue" },
  { key: "wed", label: "Wed" },
  { key: "thu", label: "Thu" },
  { key: "fri", label: "Fri" },
  { key: "sat", label: "Sat" },
  { key: "sun", label: "Sun" },
] as const;

function scheduleLabel(task: AutomationTask): string {
  if (task.task_type === "semantic") {
    if (task.trigger_type === "screen_content") return "Screen content";
    if (task.trigger_type === "url_visit") return "URL match";
    return "Trigger";
  }
  const sv = task.schedule_value ?? "";
  switch (task.schedule_type) {
    case "interval":
      return `Every ${sv} min`;
    case "daily": {
      const t = sv ? formatTime12h(sv) : "?";
      return `Daily at ${t}`;
    }
    case "weekdays": {
      const t = sv ? formatTime12h(sv) : "?";
      return `Weekdays at ${t}`;
    }
    case "specific_days": {
      const [days, time] = sv.split("|");
      const t = time ? formatTime12h(time) : "?";
      const d = days
        ?.split(",")
        .map((x) => x.charAt(0).toUpperCase() + x.slice(1))
        .join(", ");
      return `${d} at ${t}`;
    }
    default:
      return "No schedule";
  }
}

function taskTypeIcon(task: AutomationTask) {
  if (task.task_type === "semantic") return <Eye className="h-4 w-4" />;
  if (task.schedule_type === "interval") return <Clock className="h-4 w-4" />;
  return <Calendar className="h-4 w-4" />;
}

// ── Tag Input ────────────────────────────────────────────────────────

function TagInput({
  tags,
  onChange,
  placeholder,
}: {
  tags: string[];
  onChange: (tags: string[]) => void;
  placeholder: string;
}) {
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const addTag = () => {
    const val = input.trim();
    if (val && !tags.includes(val)) {
      onChange([...tags, val]);
      setInput("");
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      addTag();
    }
  };

  const removeTag = (tag: string) => {
    onChange(tags.filter((t) => t !== tag));
  };

  return (
    <div
      className="flex flex-wrap gap-1.5 rounded-md border border-input p-2 min-h-10 cursor-text bg-background"
      onClick={() => inputRef.current?.focus()}
      onKeyDown={() => {}}
    >
      {tags.map((tag) => (
        <Badge
          key={tag}
          variant="secondary"
          className="flex items-center gap-1 pr-1"
        >
          {tag}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              removeTag(tag);
            }}
            className="rounded-full hover:bg-muted-foreground/20 p-0.5"
          >
            <X className="h-3 w-3" />
          </button>
        </Badge>
      ))}
      <input
        ref={inputRef}
        className="flex-1 min-w-25 bg-transparent outline-none text-sm placeholder:text-muted-foreground"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={addTag}
        placeholder={tags.length === 0 ? placeholder : ""}
      />
    </div>
  );
}

// ── Day Selector ─────────────────────────────────────────────────────

function DaySelector({
  selected,
  onChange,
}: {
  selected: string[];
  onChange: (days: string[]) => void;
}) {
  const toggle = (day: string) => {
    if (selected.includes(day)) {
      onChange(selected.filter((d) => d !== day));
    } else {
      onChange([...selected, day]);
    }
  };

  return (
    <div className="flex gap-1">
      {ALL_DAYS.map((d) => (
        <button
          key={d.key}
          type="button"
          className={`px-2 py-1 text-xs rounded-md border transition-colors ${
            selected.includes(d.key)
              ? "bg-primary text-primary-foreground border-primary"
              : "bg-background border-input hover:bg-accent"
          }`}
          onClick={() => toggle(d.key)}
        >
          {d.label}
        </button>
      ))}
    </div>
  );
}

// ── Skill Multi-Select ───────────────────────────────────────────────

function SkillMultiSelect({
  allSkills,
  disabledSkills,
  onChange,
}: {
  allSkills: SkillSummary[];
  disabledSkills: string[];
  onChange: (disabled: string[]) => void;
}) {
  const [open, setOpen] = useState(false);

  const toggleSkill = (name: string) => {
    if (disabledSkills.includes(name)) {
      onChange(disabledSkills.filter((s) => s !== name));
    } else {
      onChange([...disabledSkills, name]);
    }
  };

  const enabledCount = allSkills.length - disabledSkills.length;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          className="w-full justify-between text-sm font-normal"
        >
          {enabledCount === allSkills.length
            ? "All tools enabled"
            : `${enabledCount}/${allSkills.length} tools enabled`}
          <ChevronDown className="h-4 w-4 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-64 p-2 max-h-60 overflow-y-auto" align="start">
        {allSkills.map((skill) => (
          <label
            key={skill.name}
            className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-accent cursor-pointer"
          >
            <Checkbox
              checked={!disabledSkills.includes(skill.name)}
              onCheckedChange={() => toggleSkill(skill.name)}
            />
            <span className="text-sm truncate">{skill.name}</span>
          </label>
        ))}
      </PopoverContent>
    </Popover>
  );
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
      await invoke("toggle_automation_task", { taskId: task.id, enabled });
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

      <AutomationDialog
        open={dialogOpen}
        onClose={handleDialogClose}
        editTask={editTask}
      />

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

          <div className="text-xs text-muted-foreground shrink-0 w-36 text-right space-y-0.5">
            <div>Last: {formatDate(task.last_run_at)}</div>
            <div>Next: {formatNextRun(task.next_run_at)}</div>
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
  const { models } = useModelAccess();

  // Available skills
  const [availableSkills, setAvailableSkills] = useState<SkillSummary[]>([]);
  useEffect(() => {
    invoke<SkillSummary[]>("get_available_skills")
      .then(setAvailableSkills)
      .catch(() => {});
  }, []);

  const enabledModels = useMemo(
    () => (models ?? []).filter((m: ModelEntry) => m.is_enabled),
    [models],
  );

  // Form state
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [taskType, setTaskType] = useState("scheduled");
  const [promptTemplate, setPromptTemplate] = useState("");
  const [scheduleType, setScheduleType] = useState("interval");
  const [scheduleValue, setScheduleValue] = useState("");
  const [timeInput, setTimeInput] = useState("");
  const [selectedDays, setSelectedDays] = useState<string[]>([]);
  const [triggerType, setTriggerType] = useState("screen_content");
  const [triggerTags, setTriggerTags] = useState<string[]>([]);
  const [maxIterations, setMaxIterations] = useState("10");
  const [timeoutSeconds, setTimeoutSeconds] = useState("120");
  const [modelId, setModelId] = useState<string>("auto");
  const [disabledSkills, setDisabledSkills] = useState<string[]>([]);
  const [notifyOnComplete, setNotifyOnComplete] = useState(true);

  // Populate form when editing
  useEffect(() => {
    if (!open) return;
    if (editTask) {
      setName(editTask.name);
      setDescription(editTask.description);
      setTaskType(editTask.task_type);
      setPromptTemplate(editTask.prompt_template);
      const st = editTask.schedule_type ?? "interval";
      setScheduleType(st);

      // Parse schedule value
      const sv = editTask.schedule_value ?? "";
      if (st === "interval") {
        setScheduleValue(sv);
        setTimeInput("");
        setSelectedDays([]);
      } else if (st === "daily" || st === "weekdays") {
        setScheduleValue("");
        setTimeInput(sv ? formatTime12h(sv) : "");
        setSelectedDays([]);
      } else if (st === "specific_days") {
        const [days, time] = sv.split("|");
        setSelectedDays(days ? days.split(",") : []);
        setTimeInput(time ? formatTime12h(time) : "");
        setScheduleValue("");
      }

      setTriggerType(editTask.trigger_type ?? "screen_content");
      // Parse trigger config
      if (editTask.trigger_config) {
        try {
          const cfg = JSON.parse(editTask.trigger_config);
          const tags =
            cfg.keywords ?? cfg.url_patterns ?? [];
          setTriggerTags(tags);
        } catch {
          setTriggerTags([]);
        }
      } else {
        setTriggerTags([]);
      }

      setMaxIterations(String(editTask.max_iterations));
      setTimeoutSeconds(String(editTask.timeout_seconds));
      setModelId(
        editTask.model_id != null ? String(editTask.model_id) : "auto",
      );
      setDisabledSkills(editTask.disabled_skills ?? []);
      setNotifyOnComplete(editTask.notify_on_complete);
    } else {
      // Reset for create
      setName("");
      setDescription("");
      setTaskType("scheduled");
      setPromptTemplate("");
      setScheduleType("interval");
      setScheduleValue("");
      setTimeInput("");
      setSelectedDays([]);
      setTriggerType("screen_content");
      setTriggerTags([]);
      setMaxIterations("10");
      setTimeoutSeconds("120");
      setModelId("auto");
      setDisabledSkills([]);
      setNotifyOnComplete(true);
    }
  }, [editTask, open]);

  // Reset schedule value when schedule type changes
  const handleScheduleTypeChange = (val: string) => {
    setScheduleType(val);
    setScheduleValue("");
    setTimeInput("");
    setSelectedDays([]);
  };

  /** Normalize the time input when user leaves the field. */
  const handleTimeBlur = () => {
    if (!timeInput.trim()) return;
    const parsed = parseTimeInput(timeInput);
    if (parsed) {
      setTimeInput(formatTime12h(parsed));
    } else {
      toast.error("Invalid time format. Try '5pm' or '5:00 PM' or '17:00'");
    }
  };

  /** Build the final schedule_value from UI state. */
  const buildScheduleValue = (): string | null => {
    if (scheduleType === "interval") {
      return scheduleValue || null;
    }
    // Parse the displayed time back to 24h
    const t = parseTimeInput(timeInput);
    if (!t) return null;

    if (scheduleType === "daily" || scheduleType === "weekdays") {
      return t;
    }
    if (scheduleType === "specific_days") {
      if (selectedDays.length === 0) return null;
      return `${selectedDays.join(",")}|${t}`;
    }
    return null;
  };

  /** Build trigger_config JSON from tags. */
  const buildTriggerConfig = (): string | null => {
    if (triggerTags.length === 0) return null;
    if (triggerType === "screen_content") {
      return JSON.stringify({ keywords: triggerTags });
    }
    if (triggerType === "url_visit") {
      return JSON.stringify({ url_patterns: triggerTags });
    }
    return null;
  };

  const handleSave = async () => {
    if (!name.trim() || !promptTemplate.trim()) {
      toast.error("Name and prompt are required");
      return;
    }

    // Validate schedule for scheduled tasks
    if (taskType === "scheduled") {
      if (scheduleType === "interval" && !scheduleValue) {
        toast.error("Please enter a number of minutes");
        return;
      }
      if (scheduleType !== "interval") {
        const parsedTime = parseTimeInput(timeInput);
        if (!parsedTime) {
          toast.error("Please enter a valid time");
          return;
        }
        if (scheduleType === "specific_days" && selectedDays.length === 0) {
          toast.error("Please select at least one day");
          return;
        }
      }
    }

    // Validate trigger for semantic tasks
    if (taskType === "semantic" && triggerTags.length === 0) {
      toast.error("Please add at least one trigger pattern");
      return;
    }

    setSaving(true);
    try {
      const resolvedModelId =
        modelId === "auto" ? null : BigInt(modelId);
      const sv = buildScheduleValue();
      const tc = buildTriggerConfig();

      if (isEdit && editTask) {
        await invoke("update_automation_task", {
          params: {
            id: editTask.id,
            name,
            description: description || null,
            prompt_template: promptTemplate,
            schedule_type: taskType === "scheduled" ? scheduleType : null,
            schedule_value: taskType === "scheduled" ? sv : null,
            trigger_type: taskType === "semantic" ? triggerType : null,
            trigger_config: taskType === "semantic" ? tc : null,
            max_iterations: Number.parseInt(maxIterations) || null,
            timeout_seconds: Number.parseInt(timeoutSeconds) || null,
            model_id: resolvedModelId,
            disabled_skills: disabledSkills,
            notify_on_complete: notifyOnComplete,
            notify_on_error: true,
            is_enabled: null,
          },
        });
        toast.success("Automation updated");
      } else {
        const params: CreateAutomationParams = {
          name,
          description: description || null,
          task_type: taskType,
          prompt_template: promptTemplate,
          model_id: resolvedModelId,
          disabled_skills: disabledSkills.length > 0 ? disabledSkills : null,
          notify_on_complete: notifyOnComplete,
          notify_on_error: true,
          max_iterations: Number.parseInt(maxIterations) || 10,
          timeout_seconds: Number.parseInt(timeoutSeconds) || 120,
          schedule_type: taskType === "scheduled" ? scheduleType : null,
          schedule_value: taskType === "scheduled" ? sv : null,
          schedule_timezone: null,
          trigger_type: taskType === "semantic" ? triggerType : null,
          trigger_config: taskType === "semantic" ? tc : null,
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
            <Label htmlFor="desc">Description</Label>
            <Input
              id="desc"
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

          {/* ── Schedule Fields ── */}
          {taskType === "scheduled" && (
            <div className="space-y-3">
              <div className="space-y-1.5">
                <Label>Schedule</Label>
                <Select
                  value={scheduleType}
                  onValueChange={handleScheduleTypeChange}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="interval">Every N minutes</SelectItem>
                    <SelectItem value="daily">Every day</SelectItem>
                    <SelectItem value="weekdays">
                      Every weekday (Mon–Fri)
                    </SelectItem>
                    <SelectItem value="specific_days">Specific days</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {scheduleType === "interval" && (
                <div className="space-y-1.5">
                  <Label>Minutes</Label>
                  <Input
                    type="number"
                    value={scheduleValue}
                    onChange={(e) => setScheduleValue(e.target.value)}
                    placeholder="30"
                    min={1}
                  />
                </div>
              )}

              {(scheduleType === "daily" || scheduleType === "weekdays") && (
                <div className="space-y-1.5">
                  <Label>Time</Label>
                  <Input
                    value={timeInput}
                    onChange={(e) => setTimeInput(e.target.value)}
                    onBlur={handleTimeBlur}
                    placeholder="5:00 PM"
                  />
                </div>
              )}

              {scheduleType === "specific_days" && (
                <>
                  <div className="space-y-1.5">
                    <Label>Days</Label>
                    <DaySelector
                      selected={selectedDays}
                      onChange={setSelectedDays}
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label>Time</Label>
                    <Input
                      value={timeInput}
                      onChange={(e) => setTimeInput(e.target.value)}
                      onBlur={handleTimeBlur}
                      placeholder="5:00 PM"
                    />
                  </div>
                </>
              )}
            </div>
          )}

          {/* ── Trigger Fields ── */}
          {taskType === "semantic" && (
            <div className="space-y-3">
              <div className="space-y-1.5">
                <Label>Trigger Type</Label>
                <Select value={triggerType} onValueChange={(v) => { setTriggerType(v); setTriggerTags([]); }}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="screen_content">
                      Screen Content
                    </SelectItem>
                    <SelectItem value="url_visit">URL Visit</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>
                  {triggerType === "screen_content"
                    ? "Match keywords"
                    : "Match URL patterns"}
                </Label>
                <TagInput
                  tags={triggerTags}
                  onChange={setTriggerTags}
                  placeholder={
                    triggerType === "screen_content"
                      ? 'Type a keyword and press Enter (e.g. "error")'
                      : 'Type a URL pattern and press Enter (e.g. "twitter.com")'
                  }
                />
                <p className="text-xs text-muted-foreground">
                  {triggerType === "screen_content"
                    ? "Triggers when any of these strings appear on screen."
                    : "Triggers when any URL containing these patterns is visible."}
                </p>
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

          {/* Model Selection */}
          <div className="space-y-1.5">
            <Label>Model</Label>
            <Select value={modelId} onValueChange={setModelId}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="auto">Auto (default model)</SelectItem>
                {enabledModels.map((m: ModelEntry) => (
                  <SelectItem key={m.id} value={String(m.id)}>
                    {m.display_name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Skill Selection */}
          {availableSkills.length > 0 && (
            <div className="space-y-1.5">
              <Label>Tools</Label>
              <SkillMultiSelect
                allSkills={availableSkills}
                disabledSkills={disabledSkills}
                onChange={setDisabledSkills}
              />
            </div>
          )}

          {/* Notify toggle */}
          <div className="flex items-center justify-between">
            <Label htmlFor="notify">Notify on completion</Label>
            <Switch
              id="notify"
              checked={notifyOnComplete}
              onCheckedChange={setNotifyOnComplete}
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

          {/* Save */}
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
    invoke<AutomationRun[]>("get_automation_runs", {
      taskId: task.id,
      limit: 20,
      offset: 0,
    })
      .then(setRuns)
      .catch((e) => console.error("Failed to fetch runs:", e))
      .finally(() => setLoading(false));
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
