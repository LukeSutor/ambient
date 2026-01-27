"use client";

import {
  AuthFooter,
  AuthFormWrapper,
  ErrorAlert,
  PasswordInput,
  SuccessCard,
  VerificationForm,
} from "@/components/auth";
import { GoogleLoginButton } from "@/components/auth/google-login-button";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { getAuthErrorMessage, useRoleAccess } from "@/lib/role-access";
import { zodResolver } from "@hookform/resolvers/zod";
import { Loader2, Mail } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

const formSchema = z.object({
  username: z.string().min(1, { message: "Username or email is required" }),
  password: z.string().min(1, { message: "Password is required" }),
});

type FormValues = z.infer<typeof formSchema>;
type FormStep = "login" | "verify" | "success";

export default function SignInPage() {
  const [isLoading, setIsLoading] = useState(false);
  const [isConfirming, setIsConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [formStep, setFormStep] = useState<FormStep>("login");
  const [verificationCode, setVerificationCode] = useState("");
  const [hasTriedConfirm, setHasTriedConfirm] = useState(false);
  const [loginData, setLoginData] = useState<{
    email: string;
    password: string;
  } | null>(null);

  const router = useRouter();
  const { signIn, confirmSignUp, resendConfirmationCode } = useRoleAccess();

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: { username: "", password: "" },
  });

  const onSubmit = async (values: FormValues) => {
    setError(null);
    setIsLoading(true);

    try {
      const result = await signIn(values.username.trim(), values.password);

      if (result.verification_required) {
        setLoginData({
          email: values.username.trim(),
          password: values.password,
        });
        setFormStep("verify");
        setVerificationCode("");
        setHasTriedConfirm(false);
      } else {
        router.push("/secondary");
      }
    } catch (err) {
      console.error("Sign in failed:", err);
      setError(
        getAuthErrorMessage(
          err,
          "Sign in failed. Please check your credentials.",
        ),
      );
    } finally {
      setIsLoading(false);
    }
  };

  const onConfirmationSubmit = async () => {
    if (!loginData) return;
    setHasTriedConfirm(true);

    if (verificationCode.length !== 8) {
      setError("Please enter the 8-digit verification code");
      return;
    }

    try {
      setIsConfirming(true);
      setError(null);
      await confirmSignUp({
        email: loginData.email,
        confirmation_code: verificationCode,
      });
      await signIn(loginData.email, loginData.password);
      setFormStep("success");
      setTimeout(() => {
        router.push("/secondary");
      }, 2000);
    } catch (err) {
      console.error("Verification failed:", err);
      setError(
        getAuthErrorMessage(err, "Verification failed. Please try again."),
      );
    } finally {
      setIsConfirming(false);
    }
  };

  const handleResendCode = async () => {
    if (!loginData) return;
    try {
      setError(null);
      await resendConfirmationCode(loginData.email);
    } catch (err) {
      console.error("Resend code failed:", err);
      setError(
        getAuthErrorMessage(err, "Failed to resend code. Please try again."),
      );
    }
  };

  const handleCodeChange = (value: string) => {
    setError(null);
    setHasTriedConfirm(false);
    setVerificationCode(value);
  };

  if (formStep === "success") {
    return (
      <SuccessCard
        title="Verification Successful!"
        description="Your email has been verified. Redirecting you now..."
        icon="loader"
      />
    );
  }

  if (formStep === "verify") {
    return (
      <VerificationForm
        email={loginData?.email || ""}
        code={verificationCode}
        onCodeChange={handleCodeChange}
        onSubmit={() => {
          void onConfirmationSubmit();
        }}
        onResendCode={() => {
          void handleResendCode();
        }}
        onBack={() => {
          setFormStep("login");
        }}
        isSubmitting={isConfirming}
        hasTriedSubmit={hasTriedConfirm}
        error={error}
        submitLabel="Verify & Sign In"
      />
    );
  }

  return (
    <AuthFormWrapper
      title="Sign In"
      description="Welcome back! Enter your credentials to continue"
      footer={
        <AuthFooter
          text="Don't have an account?"
          linkText="Create one here"
          linkHref="/secondary/signup"
        />
      }
    >
      <form
        onSubmit={(e) => {
          void form.handleSubmit(onSubmit)(e);
        }}
        className="space-y-6"
        noValidate
      >
        <ErrorAlert error={error} />

        <GoogleLoginButton
          onSignInSuccess={() => {
            router.push("/secondary");
          }}
          className="w-full mb-6"
        />

        <Controller
          control={form.control}
          name="username"
          render={({ field, fieldState }) => (
            <Field data-invalid={fieldState.invalid}>
              <FieldLabel htmlFor="signin-username">
                Username or Email
              </FieldLabel>
              <div className="relative">
                <Mail className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                <Input
                  id="signin-username"
                  className="pl-10 h-11"
                  placeholder="jane@example.com"
                  autoComplete="username"
                  disabled={isLoading}
                  aria-invalid={fieldState.invalid}
                  {...field}
                />
              </div>
              {fieldState.invalid && <FieldError errors={[fieldState.error]} />}
            </Field>
          )}
        />

        <Controller
          control={form.control}
          name="password"
          render={({ field, fieldState }) => (
            <PasswordInput
              id="signin-password"
              label="Password"
              field={field}
              fieldState={fieldState}
              disabled={isLoading}
              autoComplete="current-password"
            />
          )}
        />

        <Button
          type="submit"
          className="w-full h-11 text-base font-medium"
          disabled={isLoading}
        >
          {isLoading ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Signing in...
            </>
          ) : (
            "Sign In"
          )}
        </Button>
      </form>
    </AuthFormWrapper>
  );
}
