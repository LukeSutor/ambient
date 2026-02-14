"use client";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Field,
  FieldContent,
  FieldError,
  FieldDescription,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type {
  AddCustomModelParams,
  ModelEntry,
  UpdateCustomModelParams,
} from "@/types/models";
import { zodResolver } from "@hookform/resolvers/zod";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, Trash2 } from "lucide-react";
import { useCallback, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";
import { z } from "zod";

// ── Provider list ──────────────────────────────────────────────────
const PROVIDERS = [
  "openai",
  "google",
  "anthropic",
  "deepseek",
  "xai",
  "zai",
  "minimax",
  "qwen",
  "nvidia",
  "meta",
  "mistral",
  "microsoft",
  "huggingface",
  "openrouter",
  "groq",
  "unknown",
] as const;

const REQUEST_FORMATS = ["openai", "gemini", "anthropic"] as const;

// ── Zod schema ─────────────────────────────────────────────────────
const modelFormSchema = z.object({
  model_id: z
    .string()
    .min(1, "Model ID is required")
    .max(100, "Model ID must be at most 100 characters"),
  api_url: z.string().url("Must be a valid URL"),
  api_key: z.string().optional().or(z.literal("")),
  request_format: z.enum(REQUEST_FORMATS, {
    errorMap: () => ({ message: "Select a request format" }),
  }),
  provider: z.enum(PROVIDERS, {
    errorMap: () => ({ message: "Select a provider" }),
  }),
  display_name: z
    .string()
    .max(40, "Display name must be at most 40 characters")
    .optional()
    .or(z.literal("")),
});

type ModelFormValues = z.infer<typeof modelFormSchema>;

// ── Props ──────────────────────────────────────────────────────────
interface ModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** When editing, pass the existing model. Null/undefined = add mode. */
  model?: ModelEntry | null;
}

export function ModelDialog({
  open,
  onOpenChange,
  model,
}: ModelDialogProps) {
  const isEditing = !!model;
  const [submitting, setSubmitting] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const {
    register,
    handleSubmit,
    control,
    reset,
    formState: { errors },
  } = useForm<ModelFormValues>({
    resolver: zodResolver(modelFormSchema),
    defaultValues: model
      ? {
          model_id: model.model_id ?? model.model,
          api_url: model.api_url ?? "",
          api_key: model.api_key ?? "",
          request_format: model.request_format as (typeof REQUEST_FORMATS)[number],
          provider: model.provider as (typeof PROVIDERS)[number],
          display_name: model.display_name,
        }
      : {
          model_id: "",
          api_url: "",
          api_key: "",
          request_format: "openai",
          provider: "openai",
          display_name: "",
        },
  });

  const onSubmit = useCallback(
    async (values: ModelFormValues) => {
      setSubmitting(true);
      try {
        if (isEditing && model) {
          const params: UpdateCustomModelParams = {
            model_key: model.model,
            model_id: values.model_id,
            api_url: values.api_url,
            api_key: values.api_key ?? "",
            request_format: values.request_format,
            provider: values.provider,
            display_name: values.display_name ?? "",
          };
          await invoke("update_custom_model", { params });
          toast.success("Model updated");
        } else {
          const params: AddCustomModelParams = {
            model_id: values.model_id,
            api_url: values.api_url,
            api_key: values.api_key ?? "",
            request_format: values.request_format,
            provider: values.provider,
            display_name: values.display_name ?? "",
          };
          await invoke("add_custom_model", { params });
          toast.success("Model added");
        }
        reset();
        onOpenChange(false);
      } catch (error) {
        const msg = String(error);
        if (msg.includes("already exists")) {
          toast.error("A model with that ID already exists");
        } else {
          toast.error(msg);
        }
      } finally {
        setSubmitting(false);
      }
    },
    [isEditing, model, onOpenChange, reset],
  );

  const handleDelete = useCallback(async () => {
    if (!model) return;
    setDeleting(true);
    try {
      await invoke("delete_custom_model", { modelKey: model.model });
      toast.success("Model deleted");
      reset();
      onOpenChange(false);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setDeleting(false);
    }
  }, [model, onOpenChange, reset]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{isEditing ? "Edit Model" : "Add Custom Model"}</DialogTitle>
          <DialogDescription>
            {isEditing
              ? "Update your custom model configuration."
              : "Connect any OpenAI, Gemini, or Anthropic-compatible API."}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(onSubmit)} className="flex flex-col gap-4">
          {/* Model ID */}
          <Field data-invalid={!!errors.model_id}>
            <FieldContent>
              <Label htmlFor="model_id">Model ID</Label>
              <Input
                id="model_id"
                placeholder="gpt-4o, claude-3-5-sonnet, etc."
                {...register("model_id")}
              />
              <FieldDescription>
                The identifier sent in API requests.
              </FieldDescription>
              <FieldError errors={[errors.model_id]} />
            </FieldContent>
          </Field>

          {/* API URL */}
          <Field data-invalid={!!errors.api_url}>
            <FieldContent>
              <Label htmlFor="api_url">API URL</Label>
              <Input
                id="api_url"
                placeholder="https://api.openai.com/v1/chat/completions"
                {...register("api_url")}
              />
              <FieldDescription>
                Full endpoint URL for completions.
              </FieldDescription>
              <FieldError errors={[errors.api_url]} />
            </FieldContent>
          </Field>

          {/* API Key */}
          <Field data-invalid={!!errors.api_key}>
            <FieldContent>
              <Label htmlFor="api_key">API Key</Label>
              <Input
                id="api_key"
                type="password"
                placeholder="sk-..."
                autoComplete="off"
                {...register("api_key")}
              />
              <FieldDescription>
                Optional for localhost models. Stored encrypted in your local database.
              </FieldDescription>
              <FieldError errors={[errors.api_key]} />
            </FieldContent>
          </Field>

          {/* Request Format */}
          <Field data-invalid={!!errors.request_format}>
            <FieldContent>
              <Label>Request Format</Label>
              <Controller
                control={control}
                name="request_format"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger>
                      <SelectValue placeholder="Select format" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="openai">OpenAI</SelectItem>
                      <SelectItem value="gemini">Gemini</SelectItem>
                      <SelectItem value="anthropic">Anthropic</SelectItem>
                    </SelectContent>
                  </Select>
                )}
              />
              <FieldDescription>
                Determines how requests are formatted. Most providers use OpenAI-compatible format.
              </FieldDescription>
              <FieldError errors={[errors.request_format]} />
            </FieldContent>
          </Field>

          {/* Provider (for icon) */}
          <Field data-invalid={!!errors.provider}>
            <FieldContent>
              <Label>Provider</Label>
              <Controller
                control={control}
                name="provider"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger>
                      <SelectValue placeholder="Select provider" />
                    </SelectTrigger>
                    <SelectContent>
                      {PROVIDERS.map((p) => (
                        <SelectItem key={p} value={p} className="capitalize">
                          {p.charAt(0).toUpperCase() + p.slice(1)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
              <FieldDescription>
                Used for the provider icon in the model selector.
              </FieldDescription>
              <FieldError errors={[errors.provider]} />
            </FieldContent>
          </Field>

          {/* Display Name */}
          <Field data-invalid={!!errors.display_name}>
            <FieldContent>
              <Label htmlFor="display_name">Display Name</Label>
              <Input
                id="display_name"
                placeholder="Optional — defaults to Model ID"
                maxLength={40}
                {...register("display_name")}
              />
              <FieldError errors={[errors.display_name]} />
            </FieldContent>
          </Field>

          <DialogFooter className="flex-row justify-between gap-2">
            {isEditing && (
              <Button
                type="button"
                variant="destructive"
                size="sm"
                onClick={handleDelete}
                disabled={deleting || submitting}
              >
                {deleting ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Trash2 className="h-4 w-4" />
                )}
                Delete
              </Button>
            )}
            <div className="flex gap-2 ml-auto">
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={submitting || deleting}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={submitting || deleting}>
                {submitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {isEditing ? "Save" : "Add Model"}
              </Button>
            </div>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
