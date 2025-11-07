"use client"
import React, { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from '@/components/ui/card';
import { Field, FieldDescription, FieldError, FieldLabel } from '@/components/ui/field';
import { Controller, useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { CheckCircle, UserPlus, Loader2, Mail, Eye, EyeOff, AlertCircle, X, ArrowRight, ArrowLeft, User } from 'lucide-react';
import { useWindows } from '@/lib/windows/useWindows';
import Link from 'next/link';
import { GoogleLoginButton } from '@/components/google-login-button';
import { useRoleAccess, SignUpRequest, SignUpResult, ConfirmSignUpRequest } from '@/lib/role-access';
import { InputOTP, InputOTPGroup, InputOTPSlot } from '@/components/ui/input-otp';
import { REGEXP_ONLY_DIGITS } from 'input-otp';
import { useRouter } from 'next/navigation';

// Step 1: Personal Info (name and email)
const step1Schema = z.object({
  given_name: z.string().min(1, {
    message: "First name is required",
  }),
  family_name: z.string().min(1, {
    message: "Last name is required",
  }),
  email: z.string().email({
    message: "Please enter a valid email address",
  }),
});

// Step 2: Account Info (username and password)
const step2Schema = z.object({
  username: z.string().min(3, {
    message: "Username must be at least 3 characters long",
  }).max(20, {
    message: "Username must be less than 20 characters",
  }),
  password: z.string()
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

export default function SignUp() {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [signUpResult, setSignUpResult] = useState<SignUpResult | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);
  const [formStep, setFormStep] = useState<'step1' | 'step2' | 'verify' | 'success'>('step1');
  const [step1Data, setStep1Data] = useState<z.infer<typeof step1Schema> | null>(null);
  const [confirmationCode, setConfirmationCode] = useState("");
  const [hasTriedConfirm, setHasTriedConfirm] = useState(false);
  const router = useRouter();

  // Windows state
  const { 
    closeHUD
  } = useWindows();

  // Auth state
  const {
    isLoggedIn,
    signUp,
    signIn,
    confirmSignUp,
    resendConfirmationCode,
  } = useRoleAccess();

  const step1Form = useForm<z.infer<typeof step1Schema>>({
    resolver: zodResolver(step1Schema),
    defaultValues: {
      given_name: "",
      family_name: "",
      email: "",
    },
  });

  const step2Form = useForm<z.infer<typeof step2Schema>>({
    resolver: zodResolver(step2Schema),
    defaultValues: {
      username: "",
      password: "",
    },
  });

  // Redirect if already logged in
  useEffect(() => {
    if (isLoggedIn) {
      router.push('/hud');
    }
  }, [isLoggedIn, router]);

  const onStep1Submit = async (values: z.infer<typeof step1Schema>) => {
    setError(null);
    setStep1Data(values);
    setFormStep('step2');
  };

  const onStep2Submit = async (values: z.infer<typeof step2Schema>) => {
    if (!step1Data) {
      setError('Please complete step 1 first');
      setFormStep('step1');
      return;
    }

    try {
      setIsLoading(true);
      setError(null);
      
      const formData: SignUpRequest = {
        username: values.username,
        password: values.password,
        email: step1Data.email,
        given_name: step1Data.given_name,
        family_name: step1Data.family_name,
      };
      
      const result = await signUp(formData);
      setSignUpResult(result);
      
      if (result.user_confirmed) {
        // User is automatically confirmed, sign them in
        await signIn(values.username, values.password);
        setFormStep('success');
        setTimeout(() => {
          window.location.href = '/hud';
        }, 2000);
      } else {
        // User needs to verify email/phone
        setConfirmationCode("");
        setHasTriedConfirm(false);
        setFormStep('verify');
      }
    } catch (err) {
      console.error('Sign up failed:', err);
      // Turn err into json and extract message
      let message = 'Sign up failed.';
      try {
        const errorObj = JSON.parse(err as string);
        message = errorObj.message || message;
      } catch(err) {
        console.error('Error parsing sign-up error message:', err);
      }
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };
  
  const onConfirmationSubmit = async () => {
    if (!signUpResult) {
      setError('Sign up data not found');
      return;
    }

    setHasTriedConfirm(true);

    if (confirmationCode.length !== 6) {
      setError('Please enter the 6-digit verification code');
      return;
    }

    try {
      setIsConfirming(true);
      setError(null);
      
      const step2Values = step2Form.getValues();

      const confirmRequest: ConfirmSignUpRequest = {
        username: step2Values.username,
        confirmation_code: confirmationCode,
        session: signUpResult.session,
      };
      
      // First confirm the signup
      await confirmSignUp(confirmRequest);
      
      // Then automatically sign in the user
      await signIn(step2Values.username, step2Values.password);
      
      setFormStep('success');
      setTimeout(() => {
        window.location.href = '/hud';
      }, 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Verification failed');
    } finally {
      setIsConfirming(false);
    }
  };

  const handleResendCode = async () => {
    try {
      setError(null);
      const step2Values = step2Form.getValues();
      const result = await resendConfirmationCode(step2Values.username);
      setSignUpResult(result);
      // Show success message or update UI to indicate code was resent
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to resend code');
    }
  };

  const handleBackToStep1 = () => {
    setError(null);
    setFormStep('step1');
  };

  if (formStep === 'success') {
    return (
      <div className="relative h-full w-full">
        <Card className="relative w-full pt-12 overflow-hidden">
          {/* Drag area and close button */}
          <div data-tauri-drag-region className="absolute top-0 right-0 left-0 flex justify-end items-center py-1 pr-1 border-b">
            <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
              <X className="!h-6 !w-6" />
            </Button>
          </div>

          <CardHeader className="text-center pt-2">
            <CardTitle className="flex items-center justify-center text-green-600 text-2xl">
              <CheckCircle className="h-6 w-6 mr-2" />
              Account Created Successfully!
            </CardTitle>
            <CardDescription className="text-base">
              Your account has been created and verified. You can now access your dashboard.
            </CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <div className="animate-pulse text-sm text-gray-500">
              Redirecting to dashboard...
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (formStep === 'verify') {
    return (
      <div className="relative h-full w-full">
        <Card className="relative w-full pt-12 overflow-hidden">
          {/* Drag area and close button */}
          <div data-tauri-drag-region className="absolute top-0 right-0 left-0 flex justify-end items-center py-1 pr-1 border-b">
            <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
              <X className="!h-6 !w-6" />
            </Button>
          </div>

          <CardHeader className="text-center pt-2">
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
              <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200 mb-6">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm">{error}</span>
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
              <Field data-invalid={hasTriedConfirm && confirmationCode.length !== 6}>
                <FieldLabel htmlFor="signup-confirmation-code">
                  Verification Code
                </FieldLabel>
                <div className="flex justify-center">
                  <InputOTP
                    id="signup-confirmation-code"
                    maxLength={6}
                    pattern={REGEXP_ONLY_DIGITS}
                    value={confirmationCode}
                    onChange={(value) => {
                      console.log("Confirmation code updated:", value);
                      setError(null);
                      setHasTriedConfirm(false);
                      setConfirmationCode(value);
                    }}
                    disabled={isConfirming}
                    aria-invalid={hasTriedConfirm && confirmationCode.length !== 6}
                  >
                    <InputOTPGroup>
                      <InputOTPSlot index={0} />
                      <InputOTPSlot index={1} />
                      <InputOTPSlot index={2} />
                      <InputOTPSlot index={3} />
                      <InputOTPSlot index={4} />
                      <InputOTPSlot index={5} />
                    </InputOTPGroup>
                  </InputOTP>
                </div>
                {hasTriedConfirm && confirmationCode.length !== 6 && (
                  <FieldError
                    errors={[{ message: 'Enter the 6-digit code from your email.' }]}
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
    );
  }

  // Step 1: Personal Information
  if (formStep === 'step1') {
    return (
      <div className="h-full w-full">
        <Card className="relative w-full pt-12 overflow-auto">
          {/* Drag area and close button */}
          <div data-tauri-drag-region className="absolute top-0 right-0 left-0 flex justify-end items-center py-1 pr-1 border-b">
            <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
              <X className="!h-6 !w-6" />
            </Button>
          </div>

          <CardHeader className="text-center pt-2">
            <CardTitle className="text-3xl font-bold">
              Create Your Account
            </CardTitle>
            <CardDescription>
              Step 1 of 2: Tell us about yourself
            </CardDescription>
          </CardHeader>
          <CardContent>
            {error && (
              <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200 mb-6">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm">{error}</span>
              </div>
            )}

            <GoogleLoginButton 
              onSignInSuccess={() => window.location.href = '/hud'}
              className="w-full mb-6"
            />

            <form
              onSubmit={step1Form.handleSubmit(onStep1Submit)}
              className="space-y-6"
              noValidate
            >
              <div className="grid grid-cols-2 gap-4">
                <Controller
                  control={step1Form.control}
                  name="given_name"
                  render={({ field, fieldState }) => (
                    <Field data-invalid={fieldState.invalid}>
                      <FieldLabel htmlFor="signup-given-name">
                        First Name
                      </FieldLabel>
                      <Input
                        id="signup-given-name"
                        className="h-11"
                        placeholder="John"
                        autoComplete="given-name"
                        aria-invalid={fieldState.invalid}
                        {...field}
                      />
                      {fieldState.invalid && (
                        <FieldError errors={[fieldState.error]} />
                      )}
                    </Field>
                  )}
                />
                <Controller
                  control={step1Form.control}
                  name="family_name"
                  render={({ field, fieldState }) => (
                    <Field data-invalid={fieldState.invalid}>
                      <FieldLabel htmlFor="signup-family-name">
                        Last Name
                      </FieldLabel>
                      <Input
                        id="signup-family-name"
                        className="h-11"
                        placeholder="Doe"
                        autoComplete="family-name"
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
                control={step1Form.control}
                name="email"
                render={({ field, fieldState }) => (
                  <Field className="col-span-2" data-invalid={fieldState.invalid}>
                    <FieldLabel htmlFor="signup-email">Email</FieldLabel>
                    <div className="relative">
                      <Mail className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                      <Input
                        id="signup-email"
                        type="email"
                        className="h-11 pl-10"
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
          <CardFooter>
            <p className="text-sm text-gray-600 w-full text-center">
              Already have an account?{' '}
              <Link href="/hud/signin" className="font-medium text-blue-600 hover:text-blue-500 transition-colors">
                Sign in here
              </Link>
            </p>
          </CardFooter>
        </Card>
      </div>
    );
  }

  // Step 2: Account Information
  if (formStep === 'step2') {
    return (
      <div className="relative h-full w-full">
        <Card className="relative w-full pt-12 overflow-auto">
          {/* Drag area and close button */}
          <div data-tauri-drag-region className="absolute top-0 right-0 left-0 flex justify-end items-center py-1 pr-1 border-b">
            <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
              <X className="!h-6 !w-6" />
            </Button>
          </div>

          <CardHeader className="text-center pt-2">
            <CardTitle className="text-3xl font-bold">
              Create Your Account
            </CardTitle>
            <CardDescription>
              Step 2 of 2: Choose your credentials
            </CardDescription>
          </CardHeader>
          <CardContent>
            {error && (
              <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200 mb-6">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm">{error}</span>
              </div>
            )}

            <form
              onSubmit={step2Form.handleSubmit(onStep2Submit)}
              className="space-y-6"
              noValidate
            >
              <Controller
                control={step2Form.control}
                name="username"
                render={({ field, fieldState }) => (
                  <Field data-invalid={fieldState.invalid}>
                    <FieldLabel htmlFor="signup-username">Username</FieldLabel>
                    <div className="relative">
                      <User className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                      <Input
                        id="signup-username"
                        className="h-11 pl-10"
                        placeholder="johndoe"
                        autoComplete="username"
                        aria-invalid={fieldState.invalid}
                        disabled={isLoading}
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
                control={step2Form.control}
                name="password"
                render={({ field, fieldState }) => (
                  <Field data-invalid={fieldState.invalid}>
                    <FieldLabel htmlFor="signup-password">Password</FieldLabel>
                    <div className="relative">
                      <Input
                        id="signup-password"
                        type={showPassword ? 'text' : 'password'}
                        className="h-11 pr-10 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                        placeholder="Enter a secure password"
                        autoComplete="new-password"
                        aria-invalid={fieldState.invalid}
                        disabled={isLoading}
                        {...field}
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                        onClick={() => setShowPassword(!showPassword)}
                        disabled={isLoading}
                      >
                        {showPassword ? (
                          <EyeOff className="h-4 w-4 text-gray-400" />
                        ) : (
                          <Eye className="h-4 w-4 text-gray-400" />
                        )}
                      </Button>
                    </div>
                    <FieldDescription>
                      Use at least 8 characters including uppercase, lowercase, a number, and a special character.
                    </FieldDescription>
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
          <CardFooter>
            <p className="text-sm text-gray-600 w-full text-center">
              Already have an account?{' '}
              <Link href="/hud/signin" className="font-medium text-blue-600 hover:text-blue-500 transition-colors">
                Sign in here
              </Link>
            </p>
          </CardFooter>
        </Card>
      </div>
    );
  }

  return null;
}
