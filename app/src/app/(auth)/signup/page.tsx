"use client";
import React, { useState, useEffect } from 'react';
import { AuthService, SignUpRequest, SignUpResult } from '@/lib/auth';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { CheckCircle, UserPlus, Loader2, Mail, User, Lock, Eye, EyeOff, AlertCircle } from 'lucide-react';
import Link from 'next/link';

export default function SignUpPage() {
  const [formData, setFormData] = useState<SignUpRequest>({
    username: '',
    password: '',
    email: '',
    given_name: '',
    family_name: '',
  });
  
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPassword, setShowPassword] = useState(false);
  const [signUpResult, setSignUpResult] = useState<SignUpResult | null>(null);
  const [confirmationCode, setConfirmationCode] = useState('');
  const [isConfirming, setIsConfirming] = useState(false);
  const [step, setStep] = useState<'signup' | 'verify' | 'success'>('signup');

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

  const handleInputChange = (field: keyof SignUpRequest, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    setError(null);
  };

  const validateForm = (): boolean => {
    if (!formData.username.trim()) {
      setError('Username is required');
      return false;
    }
    if (!formData.email.trim()) {
      setError('Email is required');
      return false;
    }
    if (!formData.email.includes('@')) {
      setError('Please enter a valid email address');
      return false;
    }
    if (!formData.password.trim()) {
      setError('Password is required');
      return false;
    }
    if (formData.password.length < 8) {
      setError('Password must be at least 8 characters long');
      return false;
    }
    return true;
  };

  const handleSignUp = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) return;

    try {
      setIsLoading(true);
      setError(null);
      
      const result = await AuthService.signUp(formData);
      setSignUpResult(result);
      
      if (result.user_confirmed) {
        // User is automatically confirmed
        setStep('success');
        setTimeout(() => {
          window.location.href = '/';
        }, 3000);
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

  const handleConfirmSignUp = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!confirmationCode.trim()) {
      setError('Confirmation code is required');
      return;
    }

    if (!signUpResult) {
      setError('Sign up data not found');
      return;
    }

    try {
      setIsConfirming(true);
      setError(null);
      
      await AuthService.confirmSignUp(
        formData.username,
        confirmationCode,
        signUpResult.session
      );
      
      setStep('success');
      setTimeout(() => {
        window.location.href = '/';
      }, 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Verification failed');
    } finally {
      setIsConfirming(false);
    }
  };

  const handleResendCode = async () => {
    try {
      setError(null);
      const result = await AuthService.resendConfirmationCode(formData.username);
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
          {/* Header Section */}
          <div className="text-center">
            <h1 className="text-4xl font-bold text-gray-900 mb-4">
              Verify Your Email
            </h1>
            <p className="text-lg text-gray-600">
              We've sent a verification code to your email
            </p>
          </div>

          <Card className="w-full">
            <CardHeader className="text-center">
              <CardTitle className="flex items-center justify-center text-2xl">
                <Mail className="h-5 w-5 mr-2" />
                Email Verification
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
              
              <form onSubmit={handleConfirmSignUp} className="space-y-6">
                <div>
                  <Label htmlFor="confirmationCode" className="text-sm font-medium">Verification Code</Label>
                  <Input
                    id="confirmationCode"
                    type="text"
                    placeholder="Enter 6-digit code"
                    value={confirmationCode}
                    onChange={(e) => setConfirmationCode(e.target.value)}
                    maxLength={6}
                    className="text-center text-lg tracking-widest h-11"
                    required
                  />
                </div>
                
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

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-lg w-full space-y-8">
        {/* Header Section */}
        <div className="text-center">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Sign Up
          </h1>
          <p className="text-lg text-gray-600">
            Create your account and start automating your workflow
          </p>
        </div>

        {/* Sign Up Form */}
        <Card className="w-full">
          <CardHeader className="text-center">
            <CardTitle className="flex items-center justify-center text-2xl">
              <User className="h-5 w-5 mr-2" />
              Create Account
            </CardTitle>
            <CardDescription>
              Join Local Computer Use today
            </CardDescription>
          </CardHeader>
          <CardContent>
            {error && (
              <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
                <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
                <span className="text-red-700 text-sm">{error}</span>
              </div>
            )}
            
            <form onSubmit={handleSignUp} className="space-y-6">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="givenName" className="text-sm font-medium">First Name</Label>
                  <Input
                    id="givenName"
                    type="text"
                    placeholder="John"
                    value={formData.given_name || ''}
                    onChange={(e) => handleInputChange('given_name', e.target.value)}
                    className="h-11"
                  />
                </div>
                <div>
                  <Label htmlFor="familyName" className="text-sm font-medium">Last Name</Label>
                  <Input
                    id="familyName"
                    type="text"
                    placeholder="Doe"
                    value={formData.family_name || ''}
                    onChange={(e) => handleInputChange('family_name', e.target.value)}
                    className="h-11"
                  />
                </div>
              </div>
              
              <div>
                <Label htmlFor="username" className="text-sm font-medium">Username</Label>
                <Input
                  id="username"
                  type="text"
                  placeholder="johndoe"
                  value={formData.username}
                  onChange={(e) => handleInputChange('username', e.target.value)}
                  className="h-11"
                  required
                />
              </div>
              
              <div>
                <Label htmlFor="email" className="text-sm font-medium">Email</Label>
                <Input
                  id="email"
                  type="email"
                  placeholder="john@example.com"
                  value={formData.email}
                  onChange={(e) => handleInputChange('email', e.target.value)}
                  className="h-11"
                  required
                />
              </div>
              
              <div>
                <Label htmlFor="password" className="text-sm font-medium">Password</Label>
                <div className="relative">
                  <Input
                    id="password"
                    type={showPassword ? 'text' : 'password'}
                    placeholder="Enter your password"
                    value={formData.password}
                    onChange={(e) => handleInputChange('password', e.target.value)}
                    className="h-11 pr-10 [&::-ms-reveal]:hidden [&::-webkit-credentials-auto-fill-button]:hidden"
                    required
                    minLength={8}
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
                <p className="text-xs text-gray-500 mt-1">
                  Password must be at least 8 characters long
                </p>
              </div>
              
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
