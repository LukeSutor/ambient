"use client";
import { AuthComponent } from '@/components/auth-component';
import { ApiDemoComponent } from '@/components/api-demo-component';
import { useAuth } from '@/lib/auth';

export default function AuthDemoPage() {
  const { isAuthenticated } = useAuth();

  return (
    <div className="container mx-auto p-6">
      <div className="max-w-4xl mx-auto space-y-8">
        <div className="text-center">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            AWS Cognito Authentication Demo
          </h1>
          <p className="text-lg text-gray-600">
            Complete OAuth2 authentication with PKCE for Tauri desktop applications
          </p>
        </div>

        <div className="grid gap-8 md:grid-cols-2">
          <div>
            <h2 className="text-2xl font-semibold mb-4">Step 1: Authentication</h2>
            <AuthComponent />
          </div>

          <div>
            <h2 className="text-2xl font-semibold mb-4">Step 2: Authenticated API Calls</h2>
            <ApiDemoComponent />
          </div>
        </div>

        {isAuthenticated && (
          <div className="mt-8 p-6 bg-green-50 rounded-lg border border-green-200">
            <h3 className="text-lg font-semibold text-green-900 mb-2">
              🎉 Authentication Successful!
            </h3>
            <p className="text-green-800">
              You are now authenticated and can make secure API calls to your AWS services.
              Your tokens are securely stored in the system keyring.
            </p>
          </div>
        )}

        <div className="mt-12 space-y-6">
          <h2 className="text-2xl font-semibold">Implementation Details</h2>
          
          <div className="grid gap-6 md:grid-cols-2">
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Security Features</h3>
              <ul className="list-disc list-inside space-y-2 text-gray-700">
                <li>OAuth2 with PKCE (Proof Key for Code Exchange)</li>
                <li>CSRF token validation</li>
                <li>Secure token storage in system keyring</li>
                <li>Dynamic port allocation for callback server</li>
                <li>No client secrets stored in the application</li>
              </ul>
            </div>
            
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">Technical Stack</h3>
              <ul className="list-disc list-inside space-y-2 text-gray-700">
                <li>Tauri v2 for secure desktop integration</li>
                <li>Rust backend for OAuth2 handling</li>
                <li>Next.js frontend with TypeScript</li>
                <li>Axum web framework for callback server</li>
                <li>System keyring for secure token storage</li>
              </ul>
            </div>
          </div>

          <div className="p-6 bg-blue-50 rounded-lg border border-blue-200">
            <h3 className="text-lg font-semibold text-blue-900 mb-3">Next Steps</h3>
            <ol className="list-decimal list-inside space-y-2 text-blue-800">
              <li>Set up your AWS Cognito User Pool and App Client</li>
              <li>Configure the environment variables in <code className="bg-blue-100 px-1 rounded">src-tauri/.env</code></li>
              <li>Update the API endpoints in <code className="bg-blue-100 px-1 rounded">src/lib/api-client.ts</code></li>
              <li>Create your AWS API Gateway with Cognito authorization</li>
              <li>Deploy and test your application</li>
            </ol>
          </div>

          <div className="p-6 bg-gray-50 rounded-lg border border-gray-200">
            <h3 className="text-lg font-semibold text-gray-900 mb-3">Files Created</h3>
            <div className="grid gap-2 md:grid-cols-2 text-sm font-mono text-gray-700">
              <div>
                <p>🦀 <strong>Rust Backend:</strong></p>
                <ul className="list-disc list-inside ml-4 space-y-1">
                  <li>src-tauri/src/auth.rs</li>
                  <li>src-tauri/.env.example</li>
                </ul>
              </div>
              <div>
                <p>⚛️ <strong>Frontend:</strong></p>
                <ul className="list-disc list-inside ml-4 space-y-1">
                  <li>src/lib/auth.ts</li>
                  <li>src/lib/api-client.ts</li>
                  <li>src/components/auth-component.tsx</li>
                  <li>src/components/api-demo-component.tsx</li>
                </ul>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
