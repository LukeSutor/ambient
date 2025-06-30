"use client";
import React, { useState } from 'react';
import { AuthService, SignUpRequest, SignUpResult } from '@/lib/auth';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Loader2, Mail, User, Lock, Eye, EyeOff, CheckCircle, AlertCircle } from 'lucide-react';
import { GoogleLoginButton } from '@/components/google-login-button';

interface SignUpComponentProps {
  onSignUpSuccess?: () => void;
  onSwitchToLogin?: () => void;
}

export function SignUpComponent({ onSignUpSuccess, onSwitchToLogin }: SignUpComponentProps) {
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
  const [step, setStep] = useState<'signup' | 'verify'>('signup');

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
        onSignUpSuccess?.();
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
      
      onSignUpSuccess?.();
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

  if (step === 'verify') {
    return (
      <Card className="w-full max-w-md mx-auto">
        <CardHeader className="text-center">
          <CardTitle className="flex items-center justify-center">
            <Mail className="h-5 w-5 mr-2" />
            Verify Your Email
          </CardTitle>
          <CardDescription>
            We've sent a verification code to {signUpResult?.destination}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
              <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
              <span className="text-red-700 text-sm">{error}</span>
            </div>
          )}
          
          <form onSubmit={handleConfirmSignUp} className="space-y-4">
            <div>
              <Label htmlFor="confirmationCode">Verification Code</Label>
              <Input
                id="confirmationCode"
                type="text"
                placeholder="Enter 6-digit code"
                value={confirmationCode}
                onChange={(e) => setConfirmationCode(e.target.value)}
                maxLength={6}
                className="text-center text-lg tracking-widest"
                required
              />
            </div>
            
            <div className="space-y-3">
              <Button
                type="submit"
                className="w-full"
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
                className="w-full"
                onClick={handleResendCode}
              >
                Resend Code
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="w-full max-w-md mx-auto">
      <CardHeader className="text-center">
        <CardTitle className="flex items-center justify-center">
          <User className="h-5 w-5 mr-2" />
          Create Account
        </CardTitle>
        <CardDescription>
          Sign up for an account with AWS Cognito
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {/* Google Sign Up Button */}
          <GoogleLoginButton 
            onSignInSuccess={onSignUpSuccess}
            disabled={isLoading}
          />

          <div className="relative">
            <div className="absolute inset-0 flex items-center">
              <span className="w-full border-t" />
            </div>
            <div className="relative flex justify-center text-xs uppercase">
              <span className="bg-background px-2 text-muted-foreground">
                Or create account with email
              </span>
            </div>
          </div>
        {error && (
          <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
            <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
            <span className="text-red-700 text-sm">{error}</span>
          </div>
        )}
        
        <form onSubmit={handleSignUp} className="space-y-4">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label htmlFor="givenName">First Name</Label>
              <Input
                id="givenName"
                type="text"
                placeholder="John"
                value={formData.given_name || ''}
                onChange={(e) => handleInputChange('given_name', e.target.value)}
              />
            </div>
            <div>
              <Label htmlFor="familyName">Last Name</Label>
              <Input
                id="familyName"
                type="text"
                placeholder="Doe"
                value={formData.family_name || ''}
                onChange={(e) => handleInputChange('family_name', e.target.value)}
              />
            </div>
          </div>
          
          <div>
            <Label htmlFor="username">Username</Label>
            <Input
              id="username"
              type="text"
              placeholder="johndoe"
              value={formData.username}
              onChange={(e) => handleInputChange('username', e.target.value)}
              required
            />
          </div>
          
          <div>
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              placeholder="john@example.com"
              value={formData.email}
              onChange={(e) => handleInputChange('email', e.target.value)}
              required
            />
          </div>
          
          <div>
            <Label htmlFor="password">Password</Label>
            <div className="relative">
              <Input
                id="password"
                type={showPassword ? 'text' : 'password'}
                placeholder="Enter your password"
                value={formData.password}
                onChange={(e) => handleInputChange('password', e.target.value)}
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
            className="w-full"
            disabled={isLoading}
          >
            {isLoading ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Creating Account...
              </>
            ) : (
              <>
                <User className="h-4 w-4 mr-2" />
                Create Account
              </>
            )}
          </Button>
          
          {onSwitchToLogin && (
            <div className="text-center">
              <Button
                type="button"
                variant="link"
                onClick={onSwitchToLogin}
                className="text-sm"
              >
                Already have an account? Sign in
              </Button>
            </div>
          )}
        </form>
        </div>
      </CardContent>
    </Card>
  );
}
