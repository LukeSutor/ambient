"use client";

import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Eye, EyeOff, Lock } from "lucide-react";
import { useState } from "react";
import type { ControllerFieldState, ControllerRenderProps } from "react-hook-form";

interface PasswordInputProps {
  id: string;
  label: string;
  field: ControllerRenderProps<any, any>;
  fieldState: ControllerFieldState;
  disabled?: boolean;
  placeholder?: string;
  autoComplete?: string;
  showIcon?: boolean;
  helpText?: React.ReactNode;
}

export function PasswordInput({
  id,
  label,
  field,
  fieldState,
  disabled = false,
  placeholder,
  autoComplete = "current-password",
  showIcon = true,
  helpText,
}: PasswordInputProps) {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <Field data-invalid={fieldState.invalid}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <div className="relative">
        {showIcon && (
          <Lock className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
        )}
        <Input
          id={id}
          type={showPassword ? "text" : "password"}
          className={`${showIcon ? "pl-10" : ""} pr-10 h-11 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden`}
          placeholder={placeholder}
          autoComplete={autoComplete}
          disabled={disabled}
          aria-invalid={fieldState.invalid}
          {...field}
        />
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
          onClick={() => setShowPassword(!showPassword)}
          disabled={disabled}
        >
          {showPassword ? (
            <EyeOff className="h-4 w-4 text-gray-400" />
          ) : (
            <Eye className="h-4 w-4 text-gray-400" />
          )}
        </Button>
      </div>
      {helpText}
      {fieldState.invalid && <FieldError errors={[fieldState.error]} />}
    </Field>
  );
}
