"use client";
import React, { useEffect } from 'react';
import { AuthService, SignUpRequest, SignUpResult } from '@/lib/auth';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage, FormDescription } from '@/components/ui/form';
import { CheckCircle, UserPlus, Loader2, Mail, User, Lock, Eye, EyeOff, AlertCircle } from 'lucide-react';
import Link from 'next/link';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { useState } from 'react';

const signUpSchema = z.object({
  given_name: z.string().optional(),
  family_name: z.string().optional(),
  username: z.string().min(3, {
    message: "Username must be at least 3 characters long",
  }).max(20, {
    message: "Username must be less than 20 characters",
  }),
  email: z.string().email({
    message: "Please enter a valid email address",
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
  const [step, setStep] = useState<'signup' | 'verify' | 'success'>('signup');

  const signUpForm = useForm<z.infer<typeof signUpSchema>>({
    resolver: zodResolver(signUpSchema),
    defaultValues: {
      given_name: "",
      family_name: "",
      username: "",
      email: "",
      password: "",
    },
  });

  const confirmationForm = useForm<z.infer<typeof confirmationSchema>>({
    resolver: zodResolver(confirmationSchema),
    defaultValues: {
      confirmationCode: "",
    },
  });

  useEffect(() => {
    const checkAuth = async () => {
      try {
        const isAuthenticated = await AuthService.isAuthenticated();
        if (isAuthenticated) {
          window.location.href = '/';
        }
      } catch (error) {
        console.error('Error checking authentication:', error);
      }
    };
    
    checkAuth();
  }, []);

  const onSignUpSubmit = async (values: z.infer<typeof signUpSchema>) => {
    try {
      setIsLoading(true);
      setError(null);
      
      const formData: SignUpRequest = {
        username: values.username,
        password: values.password,
        email: values.email,
        given_name: values.given_name || '',
        family_name: values.family_name || '',
      };
      
      const result = await AuthService.signUp(formData);
      setSignUpResult(result);
      
      if (result.user_confirmed) {
        // User is automatically confirmed, sign them in
        await AuthService.signIn(values.username, values.password);
        setStep('success');
        setTimeout(() => {
          window.location.href = '/';
        }, 2000);
      } else {
        // User needs to verify email/phone
        setStep('verify');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Sign up failed');
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
      
      const signUpValues = signUpForm.getValues();
      
      // First confirm the signup
      await AuthService.confirmSignUp(
        signUpValues.username,
        values.confirmationCode,
        signUpResult.session
      );
      
      // Then automatically sign in the user
      await AuthService.signIn(signUpValues.username, signUpValues.password);
      
      setStep('success');
      setTimeout(() => {
        window.location.href = '/';
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
      const signUpValues = signUpForm.getValues();
      const result = await AuthService.resendConfirmationCode(signUpValues.username);
      setSignUpResult(result);
      // Show success message or update UI to indicate code was resent
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to resend code');
    }
  };

  if (step === 'success') {
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

  if (step === 'verify') {
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

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-lg w-full space-y-8">
        {/* Sign Up Form */}
        <Card className="w-full">
          <CardHeader className="text-center">
            <CardTitle className="flex items-center justify-center text-3xl font-bold">
              Sign Up
            </CardTitle>
            <CardDescription>
              Welcome! Create an account to join Cortical today.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {error && (
              <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
                <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
                <span className="text-red-700 text-sm">{error}</span>
              </div>
            )}
            
            <Form {...signUpForm}>
              <form onSubmit={signUpForm.handleSubmit(onSignUpSubmit)} className="space-y-6">
                <div className="grid grid-cols-2 gap-4">
                  <FormField
                    control={signUpForm.control}
                    name="given_name"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-sm font-medium">First Name</FormLabel>
                        <FormControl>
                          <Input
                            className="h-11"
                            {...field}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={signUpForm.control}
                    name="family_name"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-sm font-medium">Last Name</FormLabel>
                        <FormControl>
                          <Input
                            className="h-11"
                            {...field}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                </div>
              
                <FormField
                  control={signUpForm.control}
                  name="username"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-sm font-medium">Username</FormLabel>
                      <FormControl>
                        <Input
                          className="h-11"
                          {...field}
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                
                <FormField
                  control={signUpForm.control}
                  name="email"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-sm font-medium">Email</FormLabel>
                      <FormControl>
                        <Input
                          type="email"
                          className="h-11"
                          {...field}
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                
                <FormField
                  control={signUpForm.control}
                  name="password"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-sm font-medium">Password</FormLabel>
                      <FormControl>
                        <div className="relative">
                          <Input
                            type={showPassword ? 'text' : 'password'}
                            className="h-11 pr-10 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                            {...field}
                          />
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            className="absolute right-0 top-0 h-full px-3"
                            onClick={() => setShowPassword(!showPassword)}
                          >
                            {showPassword ? (
                              <EyeOff className="h-4 w-4" />
                            ) : (
                              <Eye className="h-4 w-4" />
                            )}
                          </Button>
                        </div>
                      </FormControl>
                      <div className="text-muted-foreground text-sm">
                        Password must be at least 8 characters long and contain:
                        <ul className="list-disc list-inside text-xs text-gray-500 mt-1 space-y-1">
                          <li>At least 1 uppercase letter</li>
                          <li>At least 1 lowercase letter</li>
                          <li>At least 1 number</li>
                          <li>At least 1 special character</li>
                        </ul>
                      </div>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              
                <Button
                  type="submit"
                  className="w-full h-11 text-base font-medium"
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
              </form>
            </Form>
          </CardContent>
        </Card>

        {/* Footer */}
        <div className="text-center">
          <p className="text-sm text-gray-600">
            Already have an account?{' '}
            <Link href="/signin" className="font-medium text-blue-600 hover:text-blue-500 transition-colors">
              Sign in here
            </Link>
          </p>
        </div>
      </div>
    </div>
  );
}
