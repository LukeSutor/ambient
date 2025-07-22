"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useForm, useFieldArray } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Plus, Minus, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { TaskService } from "@/lib/task-service";
import { TaskFrequency } from "@/types/tasks";
import { toast } from "sonner";

const taskSchema = z.object({
  name: z.string().min(1, "Task name is required"),
  description: z.string().optional(),
  frequency: z.union([
    z.literal("OneTime"),
    z.literal("Daily"),
    z.literal("Weekly"),
    z.literal("BiWeekly"),
    z.literal("Monthly"),
    z.literal("Quarterly"),
    z.literal("Yearly"),
    z.object({ Custom: z.number() })
  ]),
  steps: z.array(z.object({
    title: z.string().min(1, "Step title is required"),
    description: z.string().optional()
  })).min(1, "At least one step is required")
});

type TaskFormData = z.infer<typeof taskSchema>;

export default function NewTaskPage() {
  const router = useRouter();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const {
    register,
    control,
    handleSubmit,
    setValue,
    watch,
    formState: { errors }
  } = useForm<TaskFormData>({
    resolver: zodResolver(taskSchema),
    defaultValues: {
      name: "",
      description: "",
      frequency: "OneTime" as TaskFrequency,
      steps: [{ title: "", description: "" }]
    }
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: "steps"
  });

  const frequency = watch("frequency");

  const onSubmit = async (data: TaskFormData) => {
    try {
      setIsSubmitting(true);
      
      const createRequest = {
        name: data.name,
        description: data.description || "",
        category: null,
        priority: 1,
        frequency: data.frequency,
        steps: data.steps.map(step => ({
          title: step.title,
          description: step.description || ""
        }))
      };

      await TaskService.createTask(createRequest);
      toast.success("Task created successfully!");
      router.push("/tasks");
    } catch (error) {
      console.error("Failed to create task:", error);
      toast.error("Failed to create task");
    } finally {
      setIsSubmitting(false);
    }
  };

  const addStep = () => {
    append({ title: "", description: "" });
  };

  const removeStep = (index: number) => {
    if (fields.length > 1) {
      remove(index);
    }
  };

  return (
    <div className="container mx-auto py-8 max-w-2xl">
      {/* Header */}
      <div className="flex items-center gap-4 mb-8">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Create New Task</h1>
          <p className="text-muted-foreground">
            Set up a new task with steps and frequency
          </p>
        </div>
      </div>

      {/* Form */}
      <Card>
        <CardHeader>
          <CardTitle>Task Details</CardTitle>
          <CardDescription>
            Define your task and break it down into manageable steps
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
            {/* Task Name */}
            <div className="space-y-2">
              <Label htmlFor="name">Task Name</Label>
              <Input
                id="name"
                placeholder="Enter task name..."
                {...register("name")}
                className={errors.name ? "border-red-500" : ""}
              />
              {errors.name && (
                <p className="text-sm text-red-500">{errors.name.message}</p>
              )}
            </div>

            {/* Description */}
            <div className="space-y-2">
              <Label htmlFor="description">Description (Optional)</Label>
              <Textarea
                id="description"
                placeholder="Describe what this task involves..."
                rows={3}
                {...register("description")}
              />
            </div>

            {/* Frequency */}
            <div className="space-y-2">
              <Label htmlFor="frequency">Frequency</Label>
              <Select
                value={typeof frequency === 'string' ? frequency : 'OneTime'}
                onValueChange={(value) => setValue("frequency", value as TaskFrequency)}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select frequency" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="OneTime">One-time</SelectItem>
                  <SelectItem value="Daily">Daily</SelectItem>
                  <SelectItem value="Weekly">Weekly</SelectItem>
                  <SelectItem value="BiWeekly">Bi-weekly</SelectItem>
                  <SelectItem value="Monthly">Monthly</SelectItem>
                  <SelectItem value="Quarterly">Quarterly</SelectItem>
                  <SelectItem value="Yearly">Yearly</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* Steps */}
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <Label>Task Steps</Label>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={addStep}
                  className="flex items-center gap-2"
                >
                  <Plus className="h-4 w-4" />
                  Add Step
                </Button>
              </div>

              <div className="space-y-3">
                {fields.map((field, index) => (
                  <div key={field.id} className="flex items-start gap-2">
                    <div className="flex-1 space-y-2">
                      <Input
                        placeholder={`Step ${index + 1} title`}
                        {...register(`steps.${index}.title`)}
                        className={errors.steps?.[index]?.title ? "border-red-500" : ""}
                      />
                      {errors.steps?.[index]?.title && (
                        <p className="text-sm text-red-500">
                          {errors.steps[index]?.title?.message}
                        </p>
                      )}
                      <Input
                        placeholder={`Step ${index + 1} description (optional)`}
                        {...register(`steps.${index}.description`)}
                      />
                    </div>
                    {fields.length > 1 && (
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => removeStep(index)}
                        className="mt-0"
                      >
                        <Minus className="h-4 w-4" />
                      </Button>
                    )}
                  </div>
                ))}
              </div>

              {errors.steps && typeof errors.steps.message === 'string' && (
                <p className="text-sm text-red-500">{errors.steps.message}</p>
              )}
            </div>

            {/* Submit Buttons */}
            <div className="flex justify-end gap-4 pt-6">
              <Button
                type="button"
                variant="outline"
                onClick={() => router.back()}
                disabled={isSubmitting}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create Task"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
