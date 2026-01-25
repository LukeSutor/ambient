"use client";
import { GoogleLoginButton } from "@/components/google-login-button";
import { SiteHeader } from "@/components/site-header";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSlot,
} from "@/components/ui/input-otp";
import { getAuthErrorMessage, useRoleAccess } from "@/lib/role-access";
import { zodResolver } from "@hookform/resolvers/zod";
import { REGEXP_ONLY_DIGITS } from "input-otp";
import { AlertCircle, Eye, EyeOff, Loader2, Lock, Mail } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import React, { useEffect, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

const formSchema = z.object({
  username: z.string().min(1, {
    message: "Username or email is required",
  }),
  password: z.string().min(1, {
    message: "Password is required",
  }),
});

export default function SignInPage() {
  const [isLoading, setIsLoading] = useState(false);
  const [isConfirming, setIsConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [formStep, setFormStep] = useState<"login" | "verify" | "success">(
    "login",
  );
  const [verificationCode, setVerificationCode] = useState("");
  const [hasTriedConfirm, setHasTriedConfirm] = useState(false);
  const [loginData, setLoginData] = useState<{
    email: string;
    password: string;
  } | null>(null);
  const router = useRouter();

  const { signIn, confirmSignUp, resendConfirmationCode } = useRoleAccess();

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      username: "",
      password: "",
    },
  });

  const onSubmit = async (values: z.infer<typeof formSchema>) => {
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
        console.log("Sign in successful:", result.user);
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

      // Auto sign-in after confirmation
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

  if (formStep === "success") {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full">
          <Card className="text-center p-8">
            <CardHeader>
              <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-green-100 mb-4">
                <Loader2 className="h-6 w-6 text-green-600" />
              </div>
              <CardTitle className="text-2xl font-bold">
                Verification Successful!
              </CardTitle>
              <CardDescription>
                Your email has been verified. Redirecting you now...
              </CardDescription>
            </CardHeader>
          </Card>
        </div>
      </div>
    );
  }

  if (formStep === "verify") {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full space-y-8">
          <Card className="w-full">
            <CardHeader className="text-center">
              <CardTitle className="text-2xl font-bold">
                Verify Your Email
              </CardTitle>
              <CardDescription>
                We&apos;ve sent a code to {loginData?.email}. Enter it below to
                confirm your account.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {error && (
                <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200">
                  <AlertCircle className="h-4 w-4" />
                  <span className="text-sm">{error}</span>
                </div>
              )}

              <Field
                data-invalid={hasTriedConfirm && verificationCode.length !== 8}
              >
                <FieldLabel htmlFor="verification-code">
                  Verification Code
                </FieldLabel>
                <div className="flex justify-center">
                  <InputOTP
                    id="verification-code"
                    maxLength={8}
                    pattern={REGEXP_ONLY_DIGITS}
                    value={verificationCode}
                    onChange={(value) => {
                      setError(null);
                      setHasTriedConfirm(false);
                      setVerificationCode(value);
                    }}
                    disabled={isConfirming}
                    aria-invalid={
                      hasTriedConfirm && verificationCode.length !== 8
                    }
                  >
                    <InputOTPGroup>
                      <InputOTPSlot index={0} />
                      <InputOTPSlot index={1} />
                      <InputOTPSlot index={2} />
                      <InputOTPSlot index={3} />
                      <InputOTPSlot index={4} />
                      <InputOTPSlot index={5} />
                      <InputOTPSlot index={6} />
                      <InputOTPSlot index={7} />
                    </InputOTPGroup>
                  </InputOTP>
                </div>
                {hasTriedConfirm && verificationCode.length !== 8 && (
                  <FieldError
                    errors={[
                      { message: "Enter the 8-digit code from your email." },
                    ]}
                  />
                )}
              </Field>

              <Button
                onClick={() => {
                  void onConfirmationSubmit();
                }}
                className="w-full h-11"
                disabled={isConfirming}
              >
                {isConfirming ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Verifying...
                  </>
                ) : (
                  "Verify & Sign In"
                )}
              </Button>
            </CardContent>
            <CardHeader className="pt-0 text-center">
              <p className="text-sm text-gray-500">
                Didn&apos;t receive a code?{" "}
                <button
                  onClick={() => {
                    void handleResendCode();
                  }}
                  className="text-blue-600 hover:underline font-medium"
                  type="button"
                >
                  Resend Code
                </button>
              </p>
              <button
                onClick={() => {
                  setFormStep("login");
                }}
                className="text-sm text-gray-500 hover:text-gray-700 mt-4"
                type="button"
              >
                Back to Sign In
              </button>
            </CardHeader>
          </Card>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        {/* Sign In Form */}
        <Card className="w-full">
          <CardHeader className="text-center">
            <CardTitle className="text-3xl font-bold">Sign In</CardTitle>
            <CardDescription>
              Welcome back! Enter your credentials to continue
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form
              onSubmit={(e) => {
                void form.handleSubmit(onSubmit)(e);
              }}
              className="space-y-6"
              noValidate
            >
              {error && (
                <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200 mb-6">
                  <AlertCircle className="h-4 w-4" />
                  <span className="text-sm">{error}</span>
                </div>
              )}

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
                    {fieldState.invalid && (
                      <FieldError errors={[fieldState.error]} />
                    )}
                  </Field>
                )}
              />

              <Controller
                control={form.control}
                name="password"
                render={({ field, fieldState }) => (
                  <Field data-invalid={fieldState.invalid}>
                    <FieldLabel htmlFor="signin-password">Password</FieldLabel>
                    <div className="relative">
                      <Lock className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                      <Input
                        id="signin-password"
                        type={showPassword ? "text" : "password"}
                        className="pl-10 pr-10 h-11 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                        autoComplete="current-password"
                        disabled={isLoading}
                        aria-invalid={fieldState.invalid}
                        {...field}
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                        onClick={() => {
                          setShowPassword(!showPassword);
                        }}
                        disabled={isLoading}
                      >
                        {showPassword ? (
                          <EyeOff className="h-4 w-4 text-gray-400" />
                        ) : (
                          <Eye className="h-4 w-4 text-gray-400" />
                        )}
                      </Button>
                    </div>
                    {fieldState.invalid && (
                      <FieldError errors={[fieldState.error]} />
                    )}
                  </Field>
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
          </CardContent>
        </Card>

        {/* Footer */}
        <div className="text-center">
          <p className="text-sm text-gray-600">
            Don&apos;t have an account?{" "}
            <Link
              href="/secondary/signup"
              className="font-medium text-blue-600 hover:text-blue-500 transition-colors"
            >
              Create one here
            </Link>
          </p>
        </div>
      </div>
    </div>
  );
}
