"use client";
import { AuthComponent } from '@/components/auth-component';

export default function AuthPage() {
  return (
    <div className="container mx-auto p-6">
      <div className="max-w-2xl mx-auto">
        <h1 className="text-3xl font-bold text-center mb-8">Authentication</h1>
        <AuthComponent />
        
        <div className="mt-8 space-y-4 text-sm text-gray-600">
          <h2 className="text-lg font-semibold text-gray-900">How it works:</h2>
          <ol className="list-decimal list-inside space-y-2">
            <li>Click "Sign In with AWS Cognito" to start the authentication process</li>
            <li>Your browser will open to the AWS Cognito login page</li>
            <li>Sign in with your credentials or create a new account</li>
            <li>You'll be redirected back to the application</li>
            <li>Your authentication token will be securely stored</li>
          </ol>
          
          <div className="mt-6 p-4 bg-blue-50 rounded-lg">
            <h3 className="font-semibold text-blue-900">Security Features:</h3>
            <ul className="list-disc list-inside mt-2 text-blue-800 space-y-1">
              <li>OAuth2 with PKCE (Proof Key for Code Exchange)</li>
              <li>CSRF protection</li>
              <li>Secure token storage in system keyring</li>
              <li>No client secrets (appropriate for desktop apps)</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
