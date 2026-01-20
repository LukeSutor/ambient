"use client";

import * as React from "react";
import { useRoleAccess } from "@/lib/role-access";
import { invoke } from "@tauri-apps/api/core";
import { Minus, X } from "lucide-react";
import { AppSidebar } from "@/components/app-sidebar";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { SiteHeader } from "@/components/site-header";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";
import { useSettings } from "@/lib/settings";

export default function DashboardLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Use role access
  useRoleAccess('/secondary');
  // Use root settings
  useSettings(true);

  const handleClose = async () => {
    await invoke("close_secondary_window");
  }

  const handleMinimize = async () => {
    await invoke("minimize_secondary_window");
  }

  return (
    <div className="relative h-full max-h-[800px] w-full rounded-lg overflow-hidden border bg-white [--header-height:calc(--spacing(16))]">
      {/* Force transparent background for this window on first paint */}
      <style
        // eslint-disable-next-line react/no-danger
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
              <div className="p-4">
                {children}
              </div>
            </main>
            <Toaster richColors />
          </SidebarInset>
        </div>
      </SidebarProvider>
  </div>
  );
}
