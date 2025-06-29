# AWS Cognito Authentication Setup for Tauri

This guide will help you set up AWS Cognito authentication in your Tauri application.

## Prerequisites

1. AWS Account with access to AWS Cognito
2. Tauri application (this implementation is for Tauri v2)
3. Next.js frontend

## AWS Cognito Setup

### 1. Create a User Pool

1. Go to AWS Cognito Console
2. Click "Create user pool"
3. Configure sign-in experience:
   - Choose "Email" as the sign-in option
   - Configure password policy as needed
4. Configure security requirements:
   - Choose MFA settings (recommended: Optional)
   - Set up password recovery options
5. Configure sign-up experience:
   - Enable self-registration if needed
   - Configure required attributes (email, etc.)
6. Configure message delivery:
   - Set up email delivery (use Amazon SES for production)
7. Integrate your app:
   - Create an app client
   - **Important**: Choose "Public client" (no client secret)
   - Enable "Authorization code grant"
   - Enable "Allow Cognito Hosted UI"
8. Review and create

### 2. Configure App Client

After creating the user pool:

1. Go to your user pool → App integration tab
2. Click on your app client
3. Configure OAuth 2.0 settings:
   - **Grant types**: Authorization code grant
   - **OpenID Connect scopes**: openid, email, profile
   - **Callback URLs**: `http://localhost:PORT/callback` (PORT will be dynamic, but add common ports like 8080, 9133, etc.)
   - **Sign out URLs**: (optional)
4. Save changes

### 3. Set up Custom Domain (Optional but Recommended)

1. In your user pool → App integration → Domain name
2. Add a domain prefix (e.g., `myapp-auth`)
3. Note the full domain: `https://myapp-auth.auth.us-east-1.amazoncognito.com`

## Environment Configuration

Create a `.env` file in your `src-tauri` directory:

```env
COGNITO_CLIENT_ID=your_app_client_id_here
COGNITO_USER_POOL_DOMAIN=your_domain_prefix_here
COGNITO_REGION=us-east-1
```

### Where to find these values:

- **COGNITO_CLIENT_ID**: User Pool → App integration → App client → Client ID
- **COGNITO_USER_POOL_DOMAIN**: The domain prefix you set up (without the full URL)
- **COGNITO_REGION**: The AWS region where your user pool is created

## Implementation

The authentication implementation is already included in your project:

### Rust Backend (src-tauri/src/auth.rs)
- Handles OAuth2 flow with PKCE
- Manages token storage securely using the system keyring
- Provides Tauri commands for authentication operations

### TypeScript Frontend (src/lib/auth.ts)
- Provides `AuthService` for calling Tauri commands
- Includes `useAuth()` React hook for state management
- Type-safe interfaces for authentication data

### React Component (src/components/auth-component.tsx)
- Ready-to-use authentication UI component
- Shows login/logout buttons and user status
- Displays token information when authenticated

## Usage

### 1. Add the auth component to your app

```tsx
import { AuthComponent } from '@/components/auth-component';

export default function MyPage() {
  return (
    <div>
      <h1>My App</h1>
      <AuthComponent />
    </div>
  );
}
```

### 2. Use the auth hook in your components

```tsx
import { useAuth } from '@/lib/auth';

export function ProtectedComponent() {
  const { isAuthenticated, isLoading, token } = useAuth();

  if (isLoading) return <div>Loading...</div>;
  if (!isAuthenticated) return <div>Please log in</div>;

  return <div>Welcome! Your token: {token?.access_token}</div>;
}
```

### 3. Make authenticated API requests

```tsx
import { AuthService } from '@/lib/auth';

async function makeAuthenticatedRequest() {
  const headers = await AuthService.getAuthorizationHeader();
  
  if (!headers) {
    console.error('Not authenticated');
    return;
  }

  const response = await fetch('https://api.example.com/data', {
    headers: {
      ...headers,
      'Content-Type': 'application/json',
    },
  });

  return response.json();
}
```

## Security Considerations

1. **No Client Secret**: This implementation correctly uses PKCE without a client secret, which is appropriate for native desktop applications.

2. **Secure Token Storage**: Tokens are stored in the system keyring, which provides OS-level security.

3. **CSRF Protection**: The OAuth2 flow includes CSRF token validation.

4. **Dynamic Ports**: The callback server uses a random available port for added security.

## Troubleshooting

### Common Issues

1. **"Callback URL not allowed"**
   - Add your callback URL to the Cognito app client configuration
   - The URL format is `http://localhost:PORT/callback`

2. **"Invalid client"**
   - Check that COGNITO_CLIENT_ID is correct
   - Ensure the app client is configured as a "Public client"

3. **"Invalid scope"**
   - Verify that openid, email, and profile scopes are enabled in your app client

4. **Compilation errors**
   - Ensure all dependencies are installed: `cargo check` in src-tauri directory
   - Make sure environment variables are set

### Testing

To test the authentication:

1. Set up your environment variables
2. Run the application: `cargo tauri dev`
3. Click the "Sign In" button
4. You should be redirected to the Cognito hosted UI
5. After signing in, you should be redirected back to your app

## Production Deployment

For production deployment:

1. Set up a proper domain for your Cognito hosted UI
2. Configure proper callback URLs for your production environment
3. Use environment-specific configuration
4. Consider implementing token refresh logic
5. Set up proper logging and error handling

## Advanced Features

### Token Refresh

The current implementation stores refresh tokens but doesn't automatically refresh them. You can extend the `AuthService` to include:

```typescript
static async refreshToken(): Promise<AuthToken> {
  // Implementation for token refresh
  // This would need to be added to the Rust backend as well
}
```

### User Information

To get user information from the ID token, you can decode it:

```typescript
import { jwtDecode } from 'jwt-decode';

const userInfo = jwtDecode(token.id_token);
```

### Custom Scopes

You can request additional scopes by modifying the authentication call in `auth.rs`:

```rust
.add_scope(Scope::new("custom_scope".to_string()))
```

## Support

If you encounter issues:

1. Check the AWS Cognito documentation
2. Verify your environment configuration
3. Check the browser developer tools for network errors
4. Review the Tauri console output for error messages
