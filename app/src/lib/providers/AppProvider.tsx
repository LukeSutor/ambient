"use client";

import { ConversationProvider } from "@/lib/conversations";
import { SettingsProvider } from "@/lib/settings";
import type { ReactNode } from "react";
import { RoleAccessProvider } from "../role-access/RoleAccessProvider";
import { WindowsProvider } from "../windows/WindowsProvider";

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
      <RoleAccessProvider>
        <ConversationProvider>
          <WindowsProvider>{children}</WindowsProvider>
        </ConversationProvider>
      </RoleAccessProvider>
    </SettingsProvider>
  );
}
