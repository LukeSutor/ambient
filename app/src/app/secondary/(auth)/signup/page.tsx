"use client";
import React, { useEffect } from 'react';
import { useRouter } from 'next/router';
import { useRoleAccess, SignUpRequest, SignUpResult, ConfirmSignUpRequest } from '@/lib/role-access';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form';
import { CheckCircle, UserPlus, Loader2, Mail, User, Eye, EyeOff, AlertCircle, ArrowRight, ArrowLeft } from 'lucide-react';
import Link from 'next/link';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { useState } from 'react';
import { GoogleLoginButton } from '@/components/google-login-button';

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

const confirmationSchema = z.object({
  confirmationCode: z.string().length(6, {
    message: "Confirmation code must be 6 digits",
  }),
});

export default function SignUpPage() {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [signUpResult, setSignUpResult] = useState<SignUpResult | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);
  const [formStep, setFormStep] = useState<'step1' | 'step2' | 'verify' | 'success'>('step1');
  const [step1Data, setStep1Data] = useState<z.infer<typeof step1Schema> | null>(null);

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

  const confirmationForm = useForm<z.infer<typeof confirmationSchema>>({
    resolver: zodResolver(confirmationSchema),
    defaultValues: {
      confirmationCode: "",
    },
  });

  const router = useRouter();

  // Auth state
  const { isLoggedIn, signUp, signIn, confirmSignUp, resendConfirmationCode } = useRoleAccess();

  // Redirect if already authenticated
  useEffect(() => {
    if (isLoggedIn) {
      router.push('/secondary');
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
          router.push('/secondary');
        }, 2000);
      } else {
        // User needs to verify email/phone
        confirmationForm.reset();
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

  const onConfirmationSubmit = async (values: z.infer<typeof confirmationSchema>) => {
    if (!signUpResult) {
      setError('Sign up data not found');
      return;
    }

    try {
      setIsConfirming(true);
      setError(null);
      
      const step2Values = step2Form.getValues();

      const confirmationData: ConfirmSignUpRequest = {
        username: step2Values.username,
        confirmation_code: values.confirmationCode,
        session: signUpResult.session,
      };
      
      // First confirm the signup
      await confirmSignUp(confirmationData);
      
      // Then automatically sign in the user
      await signIn(step2Values.username, step2Values.password);

      setFormStep('success');
      setTimeout(() => {
        router.push('/secondary');
      }, 2000);
    } catch (err) {
      console.error('Verification failed:', err);
      // Turn err into json and extract message
      let message = 'Verification failed.';
      try {
        const errorObj = JSON.parse(err as string);
        message = errorObj.message || message;
      } catch(err) {
        console.error('Error parsing verification error message:', err);
      }
      setError(message);
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
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full">
          <Card>
            <CardHeader className="text-center">
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
      </div>
    );
  }

  if (formStep === 'verify') {
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
              
              <Form {...confirmationForm}>
                <form onSubmit={confirmationForm.handleSubmit(onConfirmationSubmit)} className="space-y-6">
                  <FormField
                    control={confirmationForm.control}
                    name="confirmationCode"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-sm font-medium">Verification Code</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="Enter 6-digit code"
                            maxLength={6}
                            className="text-center text-lg tracking-widest h-11"
                            autoComplete="off"
                            autoFocus
                            {...field}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  
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
              </Form>
            </CardContent>
          </Card>
        </div>
      </div>
    );
  }

  // Step 1: Personal Information
  if (formStep === 'step1') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full space-y-8">
          <Card className="w-full">
            <CardHeader className="text-center">
              <CardTitle className="text-3xl font-bold">
                Create Your Account
              </CardTitle>
              <CardDescription>
                Step 1 of 2: Tell us about yourself
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
                onSignInSuccess={() => router.push('/secondary')}
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

              <Form {...step1Form}>
                <form onSubmit={step1Form.handleSubmit(onStep1Submit)} className="space-y-6">
                  <div className="grid grid-cols-2 gap-4">
                    <FormField
                      control={step1Form.control}
                      name="given_name"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-sm font-medium">First Name</FormLabel>
                          <FormControl>
                            <Input
                              className="h-11"
                              placeholder="John"
                              {...field}
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={step1Form.control}
                      name="family_name"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel className="text-sm font-medium">Last Name</FormLabel>
                          <FormControl>
                            <Input
                              className="h-11"
                              placeholder="Doe"
                              {...field}
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                  </div>
                  
                  <FormField
                    control={step1Form.control}
                    name="email"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-sm font-medium">Email</FormLabel>
                        <FormControl>
                          <div className="relative">
                            <Mail className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                            <Input
                              type="email"
                              className="pl-10 h-11"
                              placeholder="john.doe@example.com"
                              {...field}
                            />
                          </div>
                        </FormControl>
                        <FormMessage />
                      </FormItem>
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
              </Form>
            </CardContent>
          </Card>

          {/* Footer */}
          <div className="text-center">
            <p className="text-sm text-gray-600">
              Already have an account?{' '}
              <Link href="/secondary/signin" className="font-medium text-blue-600 hover:text-blue-500 transition-colors">
                Sign in here
              </Link>
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Step 2: Account Information
  if (formStep === 'step2') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-md w-full space-y-8">
          <Card className="w-full">
            <CardHeader className="text-center">
              <CardTitle className="text-3xl font-bold">
                Create Your Account
              </CardTitle>
              <CardDescription>
                Step 2 of 2: Choose your credentials
              </CardDescription>
            </CardHeader>
            <CardContent>
              {error && (
                <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
                  <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
                  <span className="text-red-700 text-sm">{error}</span>
                </div>
              )}

              <Form {...step2Form}>
                <form onSubmit={step2Form.handleSubmit(onStep2Submit)} className="space-y-6">
                  <FormField
                    control={step2Form.control}
                    name="username"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-sm font-medium">Username</FormLabel>
                        <FormControl>
                          <div className="relative">
                            <User className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                            <Input
                              className="pl-10 h-11"
                              placeholder="johndoe"
                              disabled={isLoading}
                              {...field}
                            />
                          </div>
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  
                  <FormField
                    control={step2Form.control}
                    name="password"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-sm font-medium">Password</FormLabel>
                        <FormControl>
                          <div className="relative">
                            <Input
                              type={showPassword ? 'text' : 'password'}
                              className="h-11 pr-10 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                              placeholder="Enter a secure password"
                              disabled={isLoading}
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
                        </FormControl>
                        <div className="text-muted-foreground text-sm mt-2">
                          Password must contain:
                          <ul className="list-disc list-inside text-xs text-gray-500 mt-1 space-y-1">
                            <li>At least 8 characters</li>
                            <li>1 uppercase & 1 lowercase letter</li>
                            <li>1 number & 1 special character</li>
                          </ul>
                        </div>
                        <FormMessage />
                      </FormItem>
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
              </Form>
            </CardContent>
          </Card>

          {/* Footer */}
          <div className="text-center">
            <p className="text-sm text-gray-600">
              Already have an account?{' '}
              <Link href="/secondary/signin" className="font-medium text-blue-600 hover:text-blue-500 transition-colors">
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