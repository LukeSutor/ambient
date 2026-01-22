"use client";
import { GoogleLoginButton } from "@/components/google-login-button";
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
import {
  type ConfirmSignUpRequest,
  type SignUpRequest,
  type SignUpResponse,
  getAuthErrorMessage,
  useRoleAccess,
} from "@/lib/role-access";
import { zodResolver } from "@hookform/resolvers/zod";
import { REGEXP_ONLY_DIGITS } from "input-otp";
import {
  AlertCircle,
  ArrowLeft,
  ArrowRight,
  CheckCircle,
  Eye,
  EyeOff,
  Loader2,
  Mail,
  User,
  UserPlus,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import React, { useEffect } from "react";
import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

// Step 1: Email
const step1Schema = z.object({
  email: z.string().email({
    message: "Please enter a valid email address",
  }),
});

// Step 2: Personal Info & Password
const step2Schema = z.object({
  full_name: z.string().min(1, {
    message: "Full name is required",
  }),
  password: z
    .string()
    .min(8, {
      message: "Password must be at least 8 characters long",
    })
    .regex(/[A-Z]/, {
      message: "Password must contain at least one uppercase letter",
    })
    .regex(/[a-z]/, {
      message: "Password must contain at least one lowercase letter",
    })
    .regex(/[0-9]/, {
      message: "Password must contain at least one number",
    })
    .regex(/[\^\$\*\.\[\]\{\}\(\)\?\-"!@#%&\/\\,><':;|_~`+=\s]/, {
      message: "Password must contain at least one special character",
    }),
});

export default function SignUpPage() {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [signUpResult, setSignUpResult] = useState<SignUpResponse | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);
  const [formStep, setFormStep] = useState<
    "step1" | "step2" | "verify" | "success"
  >("step1");
  const [step1Data, setStep1Data] = useState<z.infer<
    typeof step1Schema
  > | null>(null);
  const [confirmationCode, setConfirmationCode] = useState("");
  const [hasTriedConfirm, setHasTriedConfirm] = useState(false);

  const step1Form = useForm<z.infer<typeof step1Schema>>({
    resolver: zodResolver(step1Schema),
    defaultValues: {
      email: "",
    },
  });

  const step2Form = useForm<z.infer<typeof step2Schema>>({
    resolver: zodResolver(step2Schema),
    defaultValues: {
      full_name: "",
      password: "",
    },
  });

  const router = useRouter();

  // Auth state
  const { signUp, signIn, confirmSignUp, resendConfirmationCode } =
    useRoleAccess();

  const onStep1Submit = async (values: z.infer<typeof step1Schema>) => {
    setError(null);
    setStep1Data(values);
    setFormStep("step2");
  };

  const onStep2Submit = async (values: z.infer<typeof step2Schema>) => {
    if (!step1Data) {
      setError("Please complete step 1 first");
      setFormStep("step1");
      return;
    }

    try {
      setIsLoading(true);
      setError(null);

      const formData: SignUpRequest = {
        email: step1Data.email,
        password: values.password,
        full_name: values.full_name,
      };

      const result = await signUp(formData);
      setSignUpResult(result);

      if (!result.verification_required) {
        // User is automatically confirmed, sign them in
        await signIn(step1Data.email, values.password);
        setFormStep("success");
        setTimeout(() => {
          router.push("/secondary");
        }, 2000);
      } else {
        // User needs to verify email/phone
        setConfirmationCode("");
        setHasTriedConfirm(false);
        setFormStep("verify");
      }
    } catch (err) {
      console.error("Sign up failed:", err);
      setError(getAuthErrorMessage(err, "Sign up failed. Please try again."));
    } finally {
      setIsLoading(false);
    }
  };

  const onConfirmationSubmit = async () => {
    if (!signUpResult) {
      setError("Sign up data not found");
      return;
    }

    setHasTriedConfirm(true);

    if (confirmationCode.length !== 8) {
      setError("Please enter the 8-digit verification code");
      return;
    }

    try {
      setIsConfirming(true);
      setError(null);

      const step2Values = step2Form.getValues();
      const step1Values = step1Form.getValues();

      const confirmationData: ConfirmSignUpRequest = {
        email: step1Values.email,
        confirmation_code: confirmationCode,
      };

      // First confirm the signup
      await confirmSignUp(confirmationData);

      // Then automatically sign in the user
      await signIn(step1Values.email, step2Values.password);

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
    try {
      setError(null);
      const step1Values = step1Form.getValues();
      await resendConfirmationCode(step1Values.email);
      // Show success message or update UI to indicate code was resent
      //TODO: Implement success feedback
    } catch (err) {
      console.error("Resend code failed:", err);
      setError(
        getAuthErrorMessage(err, "Failed to resend code. Please try again."),
      );
    }
  };

  const handleBackToStep1 = () => {
    setError(null);
    setFormStep("step1");
  };

  if (formStep === "success") {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full">
          <Card>
            <CardHeader className="text-center">
              <CardTitle className="flex items-center justify-center text-green-600 text-2xl">
                <CheckCircle className="h-6 w-6 mr-2" />
                Account Created Successfully!
              </CardTitle>
              <CardDescription className="text-base">
                Your account has been created and verified. You can now access
                your dashboard.
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              <div className="animate-pulse text-sm text-gray-500">
                Redirecting to dashboard...
              </div>
            </CardContent>
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
              <CardTitle className="flex items-center justify-center text-2xl">
                <Mail className="h-5 w-5 mr-2 text-3xl font-bold" />
                Verify Your Email
              </CardTitle>
              <CardDescription>
                Enter the verification code sent to {signUpResult?.destination}
              </CardDescription>
            </CardHeader>
            <CardContent>
              {error && (
                <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
                  <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
                  <span className="text-red-700 text-sm">{error}</span>
                </div>
              )}

              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  onConfirmationSubmit();
                }}
                className="space-y-6"
                noValidate
              >
                <Field
                  data-invalid={
                    hasTriedConfirm && confirmationCode.length !== 8
                  }
                >
                  <FieldLabel htmlFor="signup-confirmation-code">
                    Verification Code
                  </FieldLabel>
                  <div className="flex justify-center">
                    <InputOTP
                      id="signup-confirmation-code"
                      maxLength={8}
                      pattern={REGEXP_ONLY_DIGITS}
                      value={confirmationCode}
                      onChange={(value) => {
                        setError(null);
                        setHasTriedConfirm(false);
                        setConfirmationCode(value);
                      }}
                      disabled={isConfirming}
                      aria-invalid={
                        hasTriedConfirm && confirmationCode.length !== 8
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
                  {hasTriedConfirm && confirmationCode.length !== 8 && (
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
                    className="w-full h-11 text-base font-medium"
                    disabled={isConfirming}
                  >
                    {isConfirming ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        Verifying...
                      </>
                    ) : (
                      <>
                        <CheckCircle className="h-4 w-4 mr-2" />
                        Verify Account
                      </>
                    )}
                  </Button>

                  <Button
                    type="button"
                    variant="outline"
                    className="w-full h-11"
                    onClick={handleResendCode}
                  >
                    Resend Code
                  </Button>
                </div>
              </form>
            </CardContent>
          </Card>
        </div>
      </div>
    );
  }

  // Step 1: Email
  if (formStep === "step1") {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full space-y-8">
          <Card className="w-full">
            <CardHeader className="text-center">
              <CardTitle className="text-3xl font-bold">
                Create Your Account
              </CardTitle>
              <CardDescription>
                Step 1 of 2: Start with your email
              </CardDescription>
            </CardHeader>
            <CardContent>
              {error && (
                <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
                  <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
                  <span className="text-red-700 text-sm">{error}</span>
                </div>
              )}

              <GoogleLoginButton
                onSignInSuccess={() => router.push("/secondary")}
                className="w-full mb-6"
              />

              <div className="relative mb-6">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-2 text-muted-foreground">
                    Or continue with email
                  </span>
                </div>
              </div>

              <form
                onSubmit={step1Form.handleSubmit(onStep1Submit)}
                className="space-y-6"
                noValidate
              >
                <Controller
                  control={step1Form.control}
                  name="email"
                  render={({ field, fieldState }) => (
                    <Field data-invalid={fieldState.invalid}>
                      <FieldLabel htmlFor="signup-email">Email</FieldLabel>
                      <div className="relative">
                        <Mail className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                        <Input
                          id="signup-email"
                          type="email"
                          className="pl-10 h-11"
                          placeholder="john.doe@example.com"
                          autoComplete="email"
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

                <Button
                  type="submit"
                  className="w-full h-11 text-base font-medium"
                >
                  Continue
                  <ArrowRight className="ml-2 h-4 w-4" />
                </Button>
              </form>
            </CardContent>
          </Card>

          {/* Footer */}
          <div className="text-center">
            <p className="text-sm text-gray-600">
              Already have an account?{" "}
              <Link
                href="/secondary/signin"
                className="font-medium text-blue-600 hover:text-blue-500 transition-colors"
              >
                Sign in here
              </Link>
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Step 2: Personal Info & Password
  if (formStep === "step2") {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full space-y-8">
          <Card className="w-full">
            <CardHeader className="text-center">
              <CardTitle className="text-3xl font-bold">
                Create Your Account
              </CardTitle>
              <CardDescription>
                Step 2 of 2: Personal Info & Password
              </CardDescription>
            </CardHeader>
            <CardContent>
              {error && (
                <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
                  <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
                  <span className="text-red-700 text-sm">{error}</span>
                </div>
              )}

              <form
                onSubmit={step2Form.handleSubmit(onStep2Submit)}
                className="space-y-6"
                noValidate
              >
                <div className="space-y-4">
                  <Controller
                    control={step2Form.control}
                    name="full_name"
                    render={({ field, fieldState }) => (
                      <Field data-invalid={fieldState.invalid}>
                        <FieldLabel htmlFor="signup-full-name">
                          Full Name
                        </FieldLabel>
                        <Input
                          id="signup-full-name"
                          className="h-11"
                          placeholder="John Doe"
                          autoComplete="name"
                          disabled={isLoading}
                          aria-invalid={fieldState.invalid}
                          {...field}
                        />
                        {fieldState.invalid && (
                          <FieldError errors={[fieldState.error]} />
                        )}
                      </Field>
                    )}
                  />
                </div>

                <Controller
                  control={step2Form.control}
                  name="password"
                  render={({ field, fieldState }) => (
                    <Field data-invalid={fieldState.invalid}>
                      <FieldLabel htmlFor="signup-password">
                        Password
                      </FieldLabel>
                      <div className="relative">
                        <Input
                          id="signup-password"
                          type={showPassword ? "text" : "password"}
                          className="h-11 pr-10 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                          placeholder="Enter a secure password"
                          autoComplete="new-password"
                          disabled={isLoading}
                          aria-invalid={fieldState.invalid}
                          {...field}
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="absolute right-0 top-0 h-full px-3"
                          onClick={() => setShowPassword(!showPassword)}
                          disabled={isLoading}
                        >
                          {showPassword ? (
                            <EyeOff className="h-4 w-4" />
                          ) : (
                            <Eye className="h-4 w-4" />
                          )}
                        </Button>
                      </div>
                      <div className="text-muted-foreground text-sm mt-2">
                        Password must contain:
                        <ul className="list-disc list-inside text-xs text-gray-500 mt-1 space-y-1">
                          <li>At least 8 characters</li>
                          <li>1 uppercase & 1 lowercase letter</li>
                          <li>1 number & 1 special character</li>
                        </ul>
                      </div>
                      {fieldState.invalid && (
                        <FieldError errors={[fieldState.error]} />
                      )}
                    </Field>
                  )}
                />

                <div className="flex gap-3">
                  <Button
                    type="button"
                    variant="outline"
                    className="h-11 text-base font-medium"
                    onClick={handleBackToStep1}
                    disabled={isLoading}
                  >
                    <ArrowLeft className="mr-2 h-4 w-4" />
                    Back
                  </Button>
                  <Button
                    type="submit"
                    className="flex-1 h-11 text-base font-medium"
                    disabled={isLoading}
                  >
                    {isLoading ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        Creating Account...
                      </>
                    ) : (
                      <>
                        <UserPlus className="h-4 w-4 mr-2" />
                        Create Account
                      </>
                    )}
                  </Button>
                </div>
              </form>
            </CardContent>
          </Card>

          {/* Footer */}
          <div className="text-center">
            <p className="text-sm text-gray-600">
              Already have an account?{" "}
              <Link
                href="/secondary/signin"
                className="font-medium text-blue-600 hover:text-blue-500 transition-colors"
              >
                Sign in here
              </Link>
            </p>
          </div>
        </div>
      </div>
    );
  }

  return null;
}
