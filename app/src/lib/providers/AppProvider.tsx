'use client';

import { ReactNode } from 'react';
import { ConversationProvider } from '@/lib/conversations';
import { WindowsProvider } from '../windows/WindowsProvider';
import { SettingsProvider } from '@/lib/settings';

interface AppProviderProps {
  children: ReactNode;
}

/**
 * Composes all app-level providers into a single component
 * 
 * Provider Order (outer to inner):
 * 1. SettingsProvider - No dependencies
 * 2. ConversationProvider - May need settings in future
 */
export function AppProvider({ children }: AppProviderProps) {
  return (
    <SettingsProvider>
        <ConversationProvider>
            <WindowsProvider>
                {children}
            </WindowsProvider>
        </ConversationProvider>
    </SettingsProvider>
  );
}
