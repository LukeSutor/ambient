"use client"
import React, { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from '@/components/ui/card';
import { Controller, useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, Mail, Lock, Eye, EyeOff, AlertCircle, X } from 'lucide-react';
import { useWindows } from '@/lib/windows/useWindows';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { GoogleLoginButton } from '@/components/google-login-button';
import { useRoleAccess } from '@/lib/role-access';
import { Field, FieldError, FieldLabel } from '@/components/ui/field';
import AutoResizeContainer from '@/components/hud/auto-resize-container';
import { HudDimensions } from '@/types/settings';
import { useSettings } from '@/lib/settings/useSettings';

const formSchema = z.object({
  username: z.string().min(1, {
    message: "Username or email is required",
  }),
  password: z.string().min(1, {
    message: "Password is required",
  }),
});

export default function Login() {
  const [isLoading, setIsLoading] = useState(false);
  const [isConfirming, setIsConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [formStep, setFormStep] = useState<'login' | 'verify' | 'success'>('login');
  const [verificationCode, setVerificationCode] = useState("");
  const [loginData, setLoginData] = useState<{email: string, password: string} | null>(null);
  const router = useRouter();
  
  // Windows state
  const { 
    closeHUD
  } = useWindows();

  // Auth state
  const { signIn, confirmSignUp, resendConfirmationCode } = useRoleAccess();

  // Settings state
  const { settings, getHudDimensions } = useSettings();
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(null);
  useEffect(() => {
    (async () => {
      const dimensions = await getHudDimensions();
      setHudDimensions(dimensions);
    })();
  }, [settings]);

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
        setLoginData({ email: values.username.trim(), password: values.password });
        setFormStep('verify');
        setVerificationCode("");
      } else {
        router.push('/hud');
      }
    } catch (err) {
      console.error('Sign in failed:', err);
      // Turn err into json and extract message
      let message = 'Sign in failed. Please check your credentials.';
      console.log(err);
      try {
        const errorObj = JSON.parse(err as string);
        message = errorObj.msg || message;
      } catch(err) {}
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  const onConfirmationSubmit = async () => {
    if (!loginData) return;
    
    if (verificationCode.length !== 8) {
      setError('Please enter the 8-digit verification code');
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
      
      setFormStep('success');
      setTimeout(() => {
        router.push('/hud');
      }, 2000);
    } catch (err) {
      console.error('Verification failed:', err);
      let message = 'Verification failed.';
      try {
        const errorObj = JSON.parse(err as string);
        message = errorObj.msg || message;
      } catch(err) {}
      setError(message);
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
      console.error('Resend code failed:', err);
      // Turn err into json and extract message
      let message = 'Resend code failed.';
      try {
        const errorObj = JSON.parse(err as string);
        message = errorObj.msg || message;
      } catch(err) {
        console.error('Error parsing resend code error message:', err);
      }
      setError(message);
    }
  };

  if (formStep === 'success') {
    return (
      <AutoResizeContainer hudDimensions={hudDimensions} widthType="login" className="bg-transparent">
        <Card className="relative w-full pt-12 text-center p-8">
          <div data-tauri-drag-region className="fixed top-0 right-0 left-0 flex justify-end py-1 pr-1 items-center border-b">
            <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
              <X className="!h-6 !w-6" />
            </Button>
          </div>
          <CardHeader>
            <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-green-100 mb-4">
              <Loader2 className="h-6 w-6 text-green-600" />
            </div>
            <CardTitle className="text-2xl font-bold">Verification Successful!</CardTitle>
            <CardDescription>
              Your email has been verified. Redirecting you now...
            </CardDescription>
          </CardHeader>
        </Card>
      </AutoResizeContainer>
    );
  }

  if (formStep === 'verify') {
    return (
      <AutoResizeContainer hudDimensions={hudDimensions} widthType="login" className="bg-transparent">
        <Card className="relative w-full pt-12">
          <div data-tauri-drag-region className="fixed top-0 right-0 left-0 flex justify-end py-1 pr-1 items-center border-b">
            <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
              <X className="!h-6 !w-6" />
            </Button>
          </div>
          <CardHeader className="text-center pt-2">
            <CardTitle className="text-3xl font-bold">Verify Email</CardTitle>
            <CardDescription>
              We've sent a code to {loginData?.email}. Enter it below to confirm your account.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {error && (
              <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm">{error}</span>
              </div>
            )}
            
            <div className="space-y-2">
              <FieldLabel htmlFor="verification-code">Verification Code</FieldLabel>
              <Input
                id="verification-code"
                type="text"
                placeholder="12345678"
                className="text-center text-2xl tracking-widest h-14"
                maxLength={8}
                value={verificationCode}
                onChange={(e) => setVerificationCode(e.target.value.replace(/\D/g, ''))}
                disabled={isConfirming}
              />
            </div>

            <Button 
              onClick={onConfirmationSubmit}
              className="w-full h-11"
              disabled={isConfirming || verificationCode.length !== 8}
            >
              {isConfirming ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Verifying...
                </>
              ) : (
                'Verify & Sign In'
              )}
            </Button>
          </CardContent>
          <CardFooter className="flex-col gap-4">
            <p className="text-sm text-gray-500 text-center">
              Didn't receive a code?{' '}
              <button 
                onClick={handleResendCode}
                className="text-blue-600 hover:underline font-medium"
                type="button"
              >
                Resend Code
              </button>
            </p>
            <button
              onClick={() => setFormStep('login')}
              className="text-sm text-gray-500 hover:text-gray-700"
              type="button"
            >
              Back to Sign In
            </button>
          </CardFooter>
        </Card>
      </AutoResizeContainer>
    );
  }

  return (
    <AutoResizeContainer hudDimensions={hudDimensions} widthType="login" className="bg-transparent">
      {/* Sign In Form */}
      <Card className="relative w-full pt-12">
        {/* Drag area and close button */}
        <div data-tauri-drag-region className="fixed top-0 right-0 left-0 flex justify-end py-1 pr-1 items-center border-b">
          <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
            <X className="!h-6 !w-6" />
          </Button>
        </div>

        <CardHeader className="text-center pt-2">
          <CardTitle className="text-3xl font-bold">Sign In</CardTitle>
          <CardDescription>
            Welcome back! Enter your credentials to continue
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6" noValidate>
            {error && (
              <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200 mb-6">
                <AlertCircle className="h-4 w-4" />
                <span className="text-sm">{error}</span>
              </div>
            )}

            <GoogleLoginButton 
              onSignInSuccess={() => router.push('/hud')}
              className="w-full"
            />

            <Controller
              control={form.control}
              name="username"
              render={({ field, fieldState }) => (
                <Field data-invalid={fieldState.invalid}>
                  <FieldLabel htmlFor="login-username">Username or Email</FieldLabel>
                  <div className="relative">
                    <Mail className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                    <Input
                      id="login-username"
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
                <Field data-invalid={fieldState.invalid}>
                  <FieldLabel htmlFor="login-password">Password</FieldLabel>
                  <div className="relative">
                    <Lock className="absolute left-3 top-3 h-5 w-5 text-gray-400" />
                    <Input
                      id="login-password"
                      type={showPassword ? "text" : "password"}
                      className="pl-10 pr-10 h-11 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                      disabled={isLoading}
                      autoComplete="current-password"
                      aria-invalid={fieldState.invalid}
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
                  {fieldState.invalid && <FieldError errors={[fieldState.error]} />}
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
                'Sign In'
              )}
            </Button>
          </form>
        </CardContent>
        <CardFooter>
          <p className="text-sm text-gray-600 w-full text-center">
            Don't have an account?{' '}
            <Link href="/hud/signup" className="font-medium text-blue-600 hover:text-blue-500 transition-colors">
              Create one here
            </Link>
          </p>
        </CardFooter>
      </Card>
    </AutoResizeContainer>
  );
}