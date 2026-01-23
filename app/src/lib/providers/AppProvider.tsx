"use client";

import { ConversationProvider } from "@/lib/conversations";
import { SettingsProvider } from "@/lib/settings";
import type { ReactNode } from "react";
import { RoleAccessProvider } from "../role-access/RoleAccessProvider";
import { WindowsProvider } from "../windows/WindowsProvider";
import { SetupProvider } from "../setup/SetupProvider";

interface AppProviderProps {
  children: ReactNode;
}

/**
 * Composes all app-level providers into a single component
 */
export function AppProvider({ children }: AppProviderProps) {
  return (
    <SettingsProvider>
      <RoleAccessProvider>
        <SetupProvider>
          <ConversationProvider>
            <WindowsProvider>{children}</WindowsProvider>
          </ConversationProvider>
        </SetupProvider>
      </RoleAccessProvider>
    </SettingsProvider>
  );
}
