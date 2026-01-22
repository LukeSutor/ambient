"use client";

import { AppSidebar } from "@/components/app-sidebar";
import { SiteHeader } from "@/components/site-header";
import { Button } from "@/components/ui/button";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import { useRoleAccess } from "@/lib/role-access";
import { useSettings } from "@/lib/settings";
import { invoke } from "@tauri-apps/api/core";
import { Minus, X } from "lucide-react";
import type * as React from "react";

export default function DashboardLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Use role access
  useRoleAccess("/secondary");
  // Use root settings
  useSettings(true);

  const handleClose = async () => {
    await invoke("close_secondary_window");
  };

  const handleMinimize = async () => {
    await invoke("minimize_secondary_window");
  };

  return (
    <div className="relative h-full max-h-[800px] w-full rounded-lg overflow-hidden border bg-white [--header-height:calc(--spacing(16))]">
      {/* Force transparent background for this window on first paint */}
      <style
        /* biome-ignore lint/security/noDangerouslySetInnerHtml: Need to force transparent background for Tauri window */
        dangerouslySetInnerHTML={{
          __html:
            "html,body{background:transparent!important;background-color:transparent!important;}",
        }}
      />
      <SidebarProvider>
        <SiteHeader handleClose={handleClose} handleMinimize={handleMinimize} />
        <div className="flex flex-1">
          <AppSidebar />
          <SidebarInset>
            <main className="flex flex-1 flex-col overflow-y-auto min-h-0 max-h-[800px]">
              <div className="h-(--header-height) shrink-0" />
              <div className="p-4">{children}</div>
            </main>
            <Toaster richColors />
          </SidebarInset>
        </div>
      </SidebarProvider>
    </div>
  );
}
