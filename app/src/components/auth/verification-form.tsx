"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSlot,
} from "@/components/ui/input-otp";
import { REGEXP_ONLY_DIGITS } from "input-otp";
import { CheckCircle, Loader2, Mail } from "lucide-react";
import { ErrorAlert } from "./error-alert";

type AuthVariant = "secondary" | "hud";

interface VerificationFormProps {
  email: string;
  code: string;
  onCodeChange: (value: string) => void;
  onSubmit: () => void;
  onResendCode: () => void;
  onBack?: () => void;
  isSubmitting: boolean;
  hasTriedSubmit: boolean;
  error: string | null;
  submitLabel?: string;
  showIcon?: boolean;
  variant?: AuthVariant;
}

export function VerificationForm({
  email,
  code,
  onCodeChange,
  onSubmit,
  onResendCode,
  onBack,
  isSubmitting,
  hasTriedSubmit,
  error,
  submitLabel = "Verify",
  showIcon = false,
  variant = "secondary",
}: VerificationFormProps) {
  const isCodeInvalid = hasTriedSubmit && code.length !== 8;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit();
  };

  const cardContent = (
    <Card className={variant === "hud" ? "relative w-full pt-12" : "w-full"}>
      <CardHeader className="text-center">
        <CardTitle className={`flex items-center justify-center ${variant === "hud" ? "text-xl" : "text-2xl"} font-bold`}>
          {showIcon && <Mail className="h-5 w-5 mr-2" />}
          Verify Your Email
        </CardTitle>
        <CardDescription>
          We&apos;ve sent a code to {email}. Enter it below to confirm your
          account.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <ErrorAlert error={error} />

        <form onSubmit={handleSubmit} className="space-y-6" noValidate>
          <Field data-invalid={isCodeInvalid}>
            <FieldLabel htmlFor="verification-code">
              Verification Code
            </FieldLabel>
            <div className="flex justify-center">
              <InputOTP
                id="verification-code"
                maxLength={8}
                pattern={REGEXP_ONLY_DIGITS}
                value={code}
                onChange={onCodeChange}
                disabled={isSubmitting}
                aria-invalid={isCodeInvalid}
              >
                <InputOTPGroup>
                  {Array.from({ length: 8 }).map((_, index) => (
                    <InputOTPSlot key={index} index={index} />
                  ))}
                </InputOTPGroup>
              </InputOTP>
            </div>
            {isCodeInvalid && (
              <FieldError
                errors={[
                  { message: "Enter the 8-digit code from your email." },
                ]}
              />
            )}
          </Field>

          <div className="space-y-3">
            <Button
              type="submit"
              className="w-full h-11"
              disabled={isSubmitting}
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Verifying...
                </>
              ) : (
                <>
                  {showIcon && <CheckCircle className="h-4 w-4 mr-2" />}
                  {submitLabel}
                </>
              )}
            </Button>

            <Button
              type="button"
              variant="outline"
              className="w-full h-11"
              onClick={onResendCode}
            >
              Resend Code
            </Button>
          </div>
        </form>

        {onBack && (
          <div className="text-center pt-2">
            <button
              onClick={onBack}
              className="text-sm text-gray-500 hover:text-gray-700"
              type="button"
            >
              Back to Sign In
            </button>
          </div>
        )}
      </CardContent>
    </Card>
  );

  if (variant === "hud") {
    return cardContent;
  }

  return (
    <div className="min-h-full flex items-center justify-center bg-background py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        {cardContent}
      </div>
    </div>
  );
}
