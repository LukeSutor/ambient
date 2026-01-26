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
import {
  type ConfirmSignUpRequest,
  type SignUpRequest,
  type SignUpResponse,
  getAuthErrorMessage,
  useRoleAccess,
} from "@/lib/role-access";
import { zodResolver } from "@hookform/resolvers/zod";
import { ArrowLeft, ArrowRight, Loader2, Mail, UserPlus } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { z } from "zod";

const step1Schema = z.object({
  email: z.string().email({ message: "Please enter a valid email address" }),
});

const step2Schema = z.object({
  full_name: z.string().min(1, { message: "Full name is required" }),
  password: z
    .string()
    .min(8, { message: "Password must be at least 8 characters long" })
    .regex(/[A-Z]/, {
      message: "Password must contain at least one uppercase letter",
    })
    .regex(/[a-z]/, {
      message: "Password must contain at least one lowercase letter",
    })
    .regex(/[0-9]/, { message: "Password must contain at least one number" })
    .regex(/[\^$*.[\]{}()?\-"!@#%&/\\,><':;|_~`+=\s]/, {
      message: "Password must contain at least one special character",
    }),
});

type Step1Values = z.infer<typeof step1Schema>;
type Step2Values = z.infer<typeof step2Schema>;
type FormStep = "step1" | "step2" | "verify" | "success";

const PASSWORD_REQUIREMENTS = [
  "At least 8 characters",
  "1 uppercase & 1 lowercase letter",
  "1 number & 1 special character",
];

function PasswordHelpText() {
  return (
    <div className="text-muted-foreground text-sm mt-2">
      Password must contain:
      <ul className="list-disc list-inside text-xs text-gray-500 mt-1 space-y-1">
        {PASSWORD_REQUIREMENTS.map((req) => (
          <li key={req}>{req}</li>
        ))}
      </ul>
    </div>
  );
}

function OrDivider() {
  return (
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
  );
}

export default function SignUpPage() {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [signUpResult, setSignUpResult] = useState<SignUpResponse | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);
  const [formStep, setFormStep] = useState<FormStep>("step1");
  const [step1Data, setStep1Data] = useState<Step1Values | null>(null);
  const [confirmationCode, setConfirmationCode] = useState("");
  const [hasTriedConfirm, setHasTriedConfirm] = useState(false);

  const router = useRouter();
  const { signUp, signIn, confirmSignUp, resendConfirmationCode } =
    useRoleAccess();

  const step1Form = useForm<Step1Values>({
    resolver: zodResolver(step1Schema),
    defaultValues: { email: "" },
  });

  const step2Form = useForm<Step2Values>({
    resolver: zodResolver(step2Schema),
    defaultValues: { full_name: "", password: "" },
  });

  const onStep1Submit = async (values: Step1Values) => {
    setError(null);
    setStep1Data(values);
    setFormStep("step2");
  };

  const onStep2Submit = async (values: Step2Values) => {
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
        await signIn(step1Data.email, values.password);
        setFormStep("success");
        setTimeout(() => {
          router.push("/secondary");
        }, 2000);
      } else {
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

      await confirmSignUp(confirmationData);
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
    setConfirmationCode(value);
  };

  const handleBackToStep1 = () => {
    setError(null);
    setFormStep("step1");
  };

  if (formStep === "success") {
    return (
      <SuccessCard
        title="Account Created Successfully!"
        description="Your account has been created and verified. You can now access your dashboard."
      />
    );
  }

  if (formStep === "verify") {
    return (
      <VerificationForm
        email={signUpResult?.destination || ""}
        code={confirmationCode}
        onCodeChange={handleCodeChange}
        onSubmit={() => {
          void onConfirmationSubmit();
        }}
        onResendCode={() => {
          void handleResendCode();
        }}
        isSubmitting={isConfirming}
        hasTriedSubmit={hasTriedConfirm}
        error={error}
        submitLabel="Verify Account"
        showIcon
      />
    );
  }

  if (formStep === "step1") {
    return (
      <AuthFormWrapper
        title="Create Your Account"
        description="Step 1 of 2: Start with your email"
        footer={
          <AuthFooter
            text="Already have an account?"
            linkText="Sign in here"
            linkHref="/secondary/signin"
          />
        }
      >
        <ErrorAlert error={error} />

        <GoogleLoginButton
          onSignInSuccess={() => {
            router.push("/secondary");
          }}
          className="w-full mb-6"
        />

        <OrDivider />

        <form
          onSubmit={(e) => {
            void step1Form.handleSubmit(onStep1Submit)(e);
          }}
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

          <Button type="submit" className="w-full h-11 text-base font-medium">
            Continue
            <ArrowRight className="ml-2 h-4 w-4" />
          </Button>
        </form>
      </AuthFormWrapper>
    );
  }

  // Step 2: Personal Info & Password
  return (
    <AuthFormWrapper
      title="Create Your Account"
      description="Step 2 of 2: Personal Info & Password"
      footer={
        <AuthFooter
          text="Already have an account?"
          linkText="Sign in here"
          linkHref="/secondary/signin"
        />
      }
    >
      <ErrorAlert error={error} />

      <form
        onSubmit={(e) => {
          void step2Form.handleSubmit(onStep2Submit)(e);
        }}
        className="space-y-6"
        noValidate
      >
        <Controller
          control={step2Form.control}
          name="full_name"
          render={({ field, fieldState }) => (
            <Field data-invalid={fieldState.invalid}>
              <FieldLabel htmlFor="signup-full-name">Full Name</FieldLabel>
              <Input
                id="signup-full-name"
                className="h-11"
                placeholder="John Doe"
                autoComplete="name"
                disabled={isLoading}
                aria-invalid={fieldState.invalid}
                {...field}
              />
              {fieldState.invalid && <FieldError errors={[fieldState.error]} />}
            </Field>
          )}
        />

        <Controller
          control={step2Form.control}
          name="password"
          render={({ field, fieldState }) => (
            <PasswordInput
              id="signup-password"
              label="Password"
              field={field}
              fieldState={fieldState}
              disabled={isLoading}
              placeholder="Enter a secure password"
              autoComplete="new-password"
              showIcon={false}
              helpText={<PasswordHelpText />}
            />
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
    </AuthFormWrapper>
  );
}
