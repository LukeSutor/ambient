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
import { Field, FieldDescription, FieldError, FieldLabel } from '@/components/ui/field';

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
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const router = useRouter();
  
  // Windows state
  const { 
    closeHUD
  } = useWindows();

  // Auth state
  const {
    isLoggedIn,
    signIn
  } = useRoleAccess();

  // Redirect if already logged in
  useEffect(() => {
    if (isLoggedIn) {
      router.push('/hud');
    }
  }, [isLoggedIn, router]);

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
      console.log('Sign in successful:', result.user_info);
      router.push('/hud');
    } catch (err) {
      console.error('Sign in failed:', err);
      // Turn err into json and extract message
      let message = 'Sign in failed. Please check your credentials.';
      try {
        const errorObj = JSON.parse(err as string);
        message = errorObj.message || message;
      } catch(err) {}
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="h-full w-full">
      {/* Sign In Form */}
      <Card className="relative w-full pt-12">
        {/* Drag area and close button */}
        <div data-tauri-drag-region className="absolute top-0 right-0 left-0 flex justify-end items-center rounded-lag border-b">
          <Button className="mr-4 hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
            <X className="!h-6 !w-6" />
          </Button>
        </div>

        <CardHeader className="text-center">
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
    </div>
  );
}