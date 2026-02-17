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
import {
  Field,
  FieldContent,
  FieldError,
} from "@/components/ui/field";
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
import { zodResolver } from "@hookform/resolvers/zod";
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
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";
import { z } from "zod/v4";
import Image from "next/image";

// ── Time Helpers ─────────────────────────────────────────────────────

/**
 * Returns true if the input is syntactically valid but genuinely ambiguous.
 * Only "1200" (without am/pm) is currently flagged — it could be noon or midnight.
 */
function isAmbiguousTime(input: string): boolean {
  const s = input.trim().toLowerCase().replace(/\s+/g, "");
  return /^1200$/.test(s);
}

/**
 * Parse flexible time input → "HH:MM" 24h, or null if invalid/ambiguous.
 * Supports:
 *   "5pm"       → "17:00"
 *   "5:00 PM"   → "17:00"
 *   "520pm"     → "17:20"  (3 digits + am/pm: first digit = hours, last 2 = mins)
 *   "1030am"    → "10:30"  (4 digits + am/pm: first 2 digits = hours, last 2 = mins)
 *   "17:00"     → "17:00"
 *   "0830"      → "08:30"  (4-digit 24 h without am/pm)
 *   "1200"      → null     (ambiguous — caller should use isAmbiguousTime)
 */
function parseTimeInput(input: string): string | null {
  const s = input.trim().toLowerCase().replace(/\s+/g, "");

  // HH:MM AM/PM  e.g. "5:30pm", "10:00 AM"
  let m = s.match(/^(\d{1,2}):(\d{2})(am|pm)$/);
  if (m) {
    let h = Number.parseInt(m[1]);
    const min = Number.parseInt(m[2]);
    if (m[3] === "pm" && h < 12) h += 12;
    if (m[3] === "am" && h === 12) h = 0;
    if (h <= 23 && min <= 59)
      return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
  }

  // HHMMAM/PM without colon — 3 or 4 digits + am/pm  e.g. "520pm", "1030am"
  m = s.match(/^(\d{3,4})(am|pm)$/);
  if (m) {
    const digits = m[1];
    let h: number;
    let min: number;
    if (digits.length === 3) {
      // e.g. "520pm" → h=5, min=20
      h = Number.parseInt(digits[0]);
      min = Number.parseInt(digits.slice(1));
    } else {
      // e.g. "1030am" → h=10, min=30
      h = Number.parseInt(digits.slice(0, 2));
      min = Number.parseInt(digits.slice(2));
    }
    if (m[2] === "pm" && h < 12) h += 12;
    if (m[2] === "am" && h === 12) h = 0;
    if (h <= 23 && min <= 59)
      return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
  }

  // H or HH AM/PM  e.g. "5pm", "12am"
  m = s.match(/^(\d{1,2})(am|pm)$/);
  if (m) {
    let h = Number.parseInt(m[1]);
    if (m[2] === "pm" && h < 12) h += 12;
    if (m[2] === "am" && h === 12) h = 0;
    if (h <= 23) return `${String(h).padStart(2, "0")}:00`;
  }

  // HH:MM 24h  e.g. "17:00", "9:30"
  m = s.match(/^(\d{1,2}):(\d{2})$/);
  if (m) {
    const h = Number.parseInt(m[1]);
    const min = Number.parseInt(m[2]);
    if (h <= 23 && min <= 59)
      return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
  }

  // HHMM 24h (4-digit, no colon, no am/pm)  e.g. "0830", "1700"
  // "1200" is the only ambiguous case and is handled as error by isAmbiguousTime.
  m = s.match(/^(\d{4})$/);
  if (m) {
    if (isAmbiguousTime(s)) return null;
    const h = Number.parseInt(s.slice(0, 2));
    const min = Number.parseInt(s.slice(2));
    if (h <= 23 && min <= 59)
      return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
  }

  return null;
}

/** Returns the provider image path for a model provider string. */
function providerImage(provider: string): string {
  return provider === "unknown" ? "/logo.png" : `/providers/${provider}.webp`;
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

// ── Automation Form Schema ────────────────────────────────────────────

const automationSchema = z
  .object({
    name: z.string().min(1, "Name is required").max(100),
    description: z.string().optional().or(z.literal("")),
    taskType: z.enum(["scheduled", "semantic"]),
    promptTemplate: z.string().min(1, "Prompt is required"),
    scheduleType: z.enum(["interval", "daily", "weekdays", "specific_days"]),
    scheduleValue: z.string().optional(),
    timeInput: z.string().optional(),
    selectedDays: z.array(z.string()),
    triggerType: z.enum(["screen_content", "url_visit"]),
    triggerTags: z.array(z.string()),
    maxIterations: z.coerce
      .number()
      .int()
      .min(1, "At least 1 iteration")
      .max(50, "Max 50 iterations"),
    timeoutSeconds: z.coerce
      .number()
      .int()
      .min(10, "At least 10 seconds")
      .max(600, "Max 600 seconds"),
    modelId: z.string(),
    disabledSkills: z.array(z.string()),
    notifyOnComplete: z.boolean(),
  })
  .superRefine((data, ctx) => {
    if (data.taskType === "scheduled") {
      if (data.scheduleType === "interval") {
        const n = Number(data.scheduleValue);
        if (!data.scheduleValue || Number.isNaN(n) || n < 1) {
          ctx.addIssue({
            code: "custom",
            path: ["scheduleValue"],
            message: "Enter a valid number of minutes (min 1)",
          });
        }
      } else {
        const ti = data.timeInput?.trim() ?? "";
        if (!ti) {
          ctx.addIssue({
            code: "custom",
            path: ["timeInput"],
            message: "Time is required",
          });
        } else if (isAmbiguousTime(ti)) {
          ctx.addIssue({
            code: "custom",
            path: ["timeInput"],
            message:
              "Ambiguous time — '1200' could be noon or midnight. Use '12:00 PM' or '12:00 AM'",
          });
        } else if (!parseTimeInput(ti)) {
          ctx.addIssue({
            code: "custom",
            path: ["timeInput"],
            message: "Invalid time. Try '5pm', '5:20 PM', or '17:00'",
          });
        }
        if (
          data.scheduleType === "specific_days" &&
          data.selectedDays.length === 0
        ) {
          ctx.addIssue({
            code: "custom",
            path: ["selectedDays"],
            message: "Select at least one day",
          });
        }
      }
    }
    if (data.taskType === "semantic" && data.triggerTags.length === 0) {
      ctx.addIssue({
        code: "custom",
        path: ["triggerTags"],
        message: "Add at least one trigger pattern",
      });
    }
  });

type AutomationFormValues = z.infer<typeof automationSchema>;

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
            htmlFor={`skill-${skill.name}`}
            className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-accent cursor-pointer"
          >
            <Checkbox
              id={`skill-${skill.name}`}
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
  icon: ReactNode;
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
            {/* suppressHydrationWarning prevents locale-dependent date mismatches between SSR and client */}
            <div suppressHydrationWarning>Last: {formatDate(task.last_run_at)}</div>
            <div suppressHydrationWarning>Next: {formatNextRun(task.next_run_at)}</div>
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

  // Default model: first local (non-cloud) model
  const defaultModelId = useMemo(() => {
    const local = enabledModels.find((m: ModelEntry) => !m.is_cloud);
    return local ? String(local.id) : String(enabledModels[0]?.id ?? "");
  }, [enabledModels]);

  const buildDefaultValues = useCallback(
    (task: AutomationTask | null): AutomationFormValues => {
      if (!task) {
        return {
          name: "",
          description: "",
          taskType: "scheduled",
          promptTemplate: "",
          scheduleType: "interval",
          scheduleValue: "",
          timeInput: "",
          selectedDays: [],
          triggerType: "screen_content",
          triggerTags: [],
          maxIterations: 10,
          timeoutSeconds: 120,
          modelId: defaultModelId,
          disabledSkills: [],
          notifyOnComplete: true,
        };
      }
      const st = (task.schedule_type ?? "interval") as AutomationFormValues["scheduleType"];
      const sv = task.schedule_value ?? "";
      let scheduleValue = "";
      let timeInput = "";
      let selectedDays: string[] = [];
      if (st === "interval") {
        scheduleValue = sv;
      } else if (st === "daily" || st === "weekdays") {
        timeInput = sv ? formatTime12h(sv) : "";
      } else if (st === "specific_days") {
        const [days, time] = sv.split("|");
        selectedDays = days ? days.split(",") : [];
        timeInput = time ? formatTime12h(time) : "";
      }

      let triggerTags: string[] = [];
      if (task.trigger_config) {
        try {
          const cfg = JSON.parse(task.trigger_config);
          triggerTags = cfg.keywords ?? cfg.url_patterns ?? [];
        } catch {
          /* ignore */
        }
      }

      return {
        name: task.name,
        description: task.description ?? "",
        taskType: task.task_type as AutomationFormValues["taskType"],
        promptTemplate: task.prompt_template,
        scheduleType: st,
        scheduleValue,
        timeInput,
        selectedDays,
        triggerType: (task.trigger_type ?? "screen_content") as AutomationFormValues["triggerType"],
        triggerTags,
        maxIterations: Number(task.max_iterations),
        timeoutSeconds: Number(task.timeout_seconds),
        modelId: task.model_id != null ? String(task.model_id) : defaultModelId,
        disabledSkills: task.disabled_skills ?? [],
        notifyOnComplete: task.notify_on_complete,
      };
    },
    [defaultModelId],
  );

  const {
    register,
    handleSubmit,
    control,
    reset,
    watch,
    setValue,
    formState: { errors },
  } = useForm<AutomationFormValues>({
    // biome-ignore lint/suspicious/noExplicitAny: type incompatibility between @hookform/resolvers v5 and react-hook-form
    resolver: zodResolver(automationSchema) as any,
    defaultValues: buildDefaultValues(editTask),
  });

  // Sync form when dialog opens or editTask changes
  useEffect(() => {
    if (open) {
      reset(buildDefaultValues(editTask));
    }
  }, [open, editTask, reset, buildDefaultValues]);

  const watchedTaskType = watch("taskType");
  const watchedScheduleType = watch("scheduleType");
  const watchedTriggerType = watch("triggerType");

  const handleScheduleTypeChange = (val: string) => {
    setValue("scheduleType", val as AutomationFormValues["scheduleType"]);
    setValue("scheduleValue", "");
    setValue("timeInput", "");
    setValue("selectedDays", []);
  };

  /** Auto-format the time field on blur if unambiguous; leave ambiguous/invalid for Zod to report. */
  const handleTimeBlur = useCallback(() => {
    const ti = watch("timeInput")?.trim() ?? "";
    if (!ti || isAmbiguousTime(ti)) return;
    const parsed = parseTimeInput(ti);
    if (parsed) {
      setValue("timeInput", formatTime12h(parsed), { shouldValidate: false });
    }
  }, [watch, setValue]);

  const buildScheduleValue = (data: AutomationFormValues): string | null => {
    if (data.scheduleType === "interval") return data.scheduleValue || null;
    const t = parseTimeInput(data.timeInput ?? "");
    if (!t) return null;
    if (data.scheduleType === "daily" || data.scheduleType === "weekdays")
      return t;
    if (data.scheduleType === "specific_days") {
      if (data.selectedDays.length === 0) return null;
      return `${data.selectedDays.join(",")}|${t}`;
    }
    return null;
  };

  const buildTriggerConfig = (data: AutomationFormValues): string | null => {
    if (data.triggerTags.length === 0) return null;
    if (data.triggerType === "screen_content")
      return JSON.stringify({ keywords: data.triggerTags });
    return JSON.stringify({ url_patterns: data.triggerTags });
  };

  const onSubmit = async (data: AutomationFormValues) => {
    setSaving(true);
    try {
      // Default to local model (id=1) if modelId is somehow not set
      const resolvedModelId = data.modelId
        ? Number.parseInt(data.modelId)
        : 1;
      const sv = buildScheduleValue(data);
      const tc = buildTriggerConfig(data);

      if (isEdit && editTask) {
        await invoke("update_automation_task", {
          params: {
            id: editTask.id,
            name: data.name,
            description: data.description || null,
            prompt_template: data.promptTemplate,
            schedule_type: data.taskType === "scheduled" ? data.scheduleType : null,
            schedule_value: data.taskType === "scheduled" ? sv : null,
            trigger_type: data.taskType === "semantic" ? data.triggerType : null,
            trigger_config: data.taskType === "semantic" ? tc : null,
            max_iterations: data.maxIterations,
            timeout_seconds: data.timeoutSeconds,
            model_id: resolvedModelId,
            disabled_skills: data.disabledSkills,
            notify_on_complete: data.notifyOnComplete,
            notify_on_error: true,
            is_enabled: null,
          },
        });
        toast.success("Automation updated");
      } else {
        const params = {
          name: data.name,
          description: data.description || null,
          task_type: data.taskType,
          prompt_template: data.promptTemplate,
          model_id: resolvedModelId,
          disabled_skills: data.disabledSkills.length > 0 ? data.disabledSkills : null,
          notify_on_complete: data.notifyOnComplete,
          notify_on_error: true,
          max_iterations: data.maxIterations,
          timeout_seconds: data.timeoutSeconds,
          schedule_type: data.taskType === "scheduled" ? data.scheduleType : null,
          schedule_value: data.taskType === "scheduled" ? sv : null,
          schedule_timezone: null,
          trigger_type: data.taskType === "semantic" ? data.triggerType : null,
          trigger_config: data.taskType === "semantic" ? tc : null,
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

        <form
          onSubmit={(e) => {
            void handleSubmit(onSubmit)(e);
          }}
          className="space-y-4 mt-2"
        >
          {/* Name */}
          <Field data-invalid={!!errors.name}>
            <FieldContent>
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                placeholder="Daily Summary"
                {...register("name")}
              />
              <FieldError errors={[errors.name]} />
            </FieldContent>
          </Field>

          {/* Description */}
          <Field>
            <FieldContent>
              <Label htmlFor="desc">Description</Label>
              <Input
                id="desc"
                placeholder="A brief description of what this does"
                {...register("description")}
              />
            </FieldContent>
          </Field>

          {/* Task Type */}
          <Field>
            <FieldContent>
              <Label>Type</Label>
              <Controller
                name="taskType"
                control={control}
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
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
                )}
              />
            </FieldContent>
          </Field>

          {/* ── Schedule Fields ── */}
          {watchedTaskType === "scheduled" && (
            <div className="space-y-3">
              <Field>
                <FieldContent>
                  <Label>Schedule</Label>
                  <Controller
                    name="scheduleType"
                    control={control}
                    render={({ field }) => (
                      <Select
                        value={field.value}
                        onValueChange={(v) => {
                          field.onChange(v);
                          handleScheduleTypeChange(v);
                        }}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="interval">
                            Every N minutes
                          </SelectItem>
                          <SelectItem value="daily">Every day</SelectItem>
                          <SelectItem value="weekdays">
                            Every weekday (Mon–Fri)
                          </SelectItem>
                          <SelectItem value="specific_days">
                            Specific days
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    )}
                  />
                </FieldContent>
              </Field>

              {watchedScheduleType === "interval" && (
                <Field data-invalid={!!errors.scheduleValue}>
                  <FieldContent>
                    <Label>Minutes</Label>
                    <Input
                      type="number"
                      placeholder="30"
                      min={1}
                      {...register("scheduleValue")}
                    />
                    <FieldError errors={[errors.scheduleValue]} />
                  </FieldContent>
                </Field>
              )}

              {(watchedScheduleType === "daily" ||
                watchedScheduleType === "weekdays") && (
                <Field data-invalid={!!errors.timeInput}>
                  <FieldContent>
                    <Label>Time</Label>
                    <Controller
                      name="timeInput"
                      control={control}
                      render={({ field }) => (
                        <Input
                          placeholder="5:00 PM"
                          value={field.value ?? ""}
                          onChange={field.onChange}
                          onBlur={() => {
                            field.onBlur();
                            handleTimeBlur();
                          }}
                        />
                      )}
                    />
                    <FieldError errors={[errors.timeInput]} />
                  </FieldContent>
                </Field>
              )}

              {watchedScheduleType === "specific_days" && (
                <>
                  <Field data-invalid={!!errors.selectedDays}>
                    <FieldContent>
                      <Label>Days</Label>
                      <Controller
                        name="selectedDays"
                        control={control}
                        render={({ field }) => (
                          <DaySelector
                            selected={field.value}
                            onChange={field.onChange}
                          />
                        )}
                      />
                      <FieldError errors={[errors.selectedDays?.root]} />
                    </FieldContent>
                  </Field>
                  <Field data-invalid={!!errors.timeInput}>
                    <FieldContent>
                      <Label>Time</Label>
                      <Controller
                        name="timeInput"
                        control={control}
                        render={({ field }) => (
                          <Input
                            placeholder="5:00 PM"
                            value={field.value ?? ""}
                            onChange={field.onChange}
                            onBlur={() => {
                              field.onBlur();
                              handleTimeBlur();
                            }}
                          />
                        )}
                      />
                      <FieldError errors={[errors.timeInput]} />
                    </FieldContent>
                  </Field>
                </>
              )}
            </div>
          )}

          {/* ── Trigger Fields ── */}
          {watchedTaskType === "semantic" && (
            <div className="space-y-3">
              <Field>
                <FieldContent>
                  <Label>Trigger Type</Label>
                  <Controller
                    name="triggerType"
                    control={control}
                    render={({ field }) => (
                      <Select
                        value={field.value}
                        onValueChange={(v) => {
                          field.onChange(v);
                          setValue("triggerTags", []);
                        }}
                      >
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
                    )}
                  />
                </FieldContent>
              </Field>
              <Field data-invalid={!!errors.triggerTags}>
                <FieldContent>
                  <Label>
                    {watchedTriggerType === "screen_content"
                      ? "Match keywords"
                      : "Match URL patterns"}
                  </Label>
                  <Controller
                    name="triggerTags"
                    control={control}
                    render={({ field }) => (
                      <TagInput
                        tags={field.value}
                        onChange={field.onChange}
                        placeholder={
                          watchedTriggerType === "screen_content"
                            ? 'Type a keyword and press Enter (e.g. "error")'
                            : 'Type a URL pattern and press Enter (e.g. "twitter.com")'
                        }
                      />
                    )}
                  />
                  <p className="text-xs text-muted-foreground">
                    {watchedTriggerType === "screen_content"
                      ? "Triggers when any of these strings appear on screen."
                      : "Triggers when any URL containing these patterns is visible."}
                  </p>
                  <FieldError errors={[errors.triggerTags?.root]} />
                </FieldContent>
              </Field>
            </div>
          )}

          {/* Prompt */}
          <Field data-invalid={!!errors.promptTemplate}>
            <FieldContent>
              <Label htmlFor="prompt">Prompt</Label>
              <Textarea
                id="prompt"
                placeholder="What should this automation do when it runs?"
                rows={4}
                {...register("promptTemplate")}
              />
              <FieldError errors={[errors.promptTemplate]} />
            </FieldContent>
          </Field>

          {/* Model Selection */}
          <Field>
            <FieldContent>
              <Label>Model</Label>
              <Controller
                name="modelId"
                control={control}
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {enabledModels.map((m: ModelEntry) => (
                        <SelectItem key={m.id} value={String(m.id)}>
                          <div className="flex items-center gap-2">
                            <Image
                              src={providerImage(m.provider)}
                              width={16}
                              height={16}
                              className="rounded-sm shrink-0"
                              alt={m.provider}
                            />
                            {m.display_name}
                          </div>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
            </FieldContent>
          </Field>

          {/* Skill Selection */}
          {availableSkills.length > 0 && (
            <Field>
              <FieldContent>
                <Label>Tools</Label>
                <Controller
                  name="disabledSkills"
                  control={control}
                  render={({ field }) => (
                    <SkillMultiSelect
                      allSkills={availableSkills}
                      disabledSkills={field.value}
                      onChange={field.onChange}
                    />
                  )}
                />
              </FieldContent>
            </Field>
          )}

          {/* Notify toggle */}
          <div className="flex items-center justify-between">
            <Label htmlFor="notify">Notify on completion</Label>
            <Controller
              name="notifyOnComplete"
              control={control}
              render={({ field }) => (
                <Switch
                  id="notify"
                  checked={field.value}
                  onCheckedChange={field.onChange}
                />
              )}
            />
          </div>

          {/* Advanced */}
          <div className="grid grid-cols-2 gap-3">
            <Field data-invalid={!!errors.maxIterations}>
              <FieldContent>
                <Label>Max Iterations</Label>
                <Input
                  type="number"
                  min={1}
                  max={50}
                  {...register("maxIterations")}
                />
                <FieldError errors={[errors.maxIterations]} />
              </FieldContent>
            </Field>
            <Field data-invalid={!!errors.timeoutSeconds}>
              <FieldContent>
                <Label>Timeout (seconds)</Label>
                <Input
                  type="number"
                  min={10}
                  max={600}
                  {...register("timeoutSeconds")}
                />
                <FieldError errors={[errors.timeoutSeconds]} />
              </FieldContent>
            </Field>
          </div>

          {/* Save */}
          <div className="flex justify-end pt-2">
            <Button type="submit" disabled={saving}>
              {saving && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              {isEdit ? "Save Changes" : "Create Automation"}
            </Button>
          </div>
        </form>
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
                      <span className="text-xs text-muted-foreground" suppressHydrationWarning>
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
