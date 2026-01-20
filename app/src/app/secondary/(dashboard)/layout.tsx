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
    <div className="relative h-full max-h-[800px] w-full rounded-lg overflow-hidden border">
      {/* Force transparent background for this window on first paint */}
      <style
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{
          __html:
            "html,body{background:transparent!important;background-color:transparent!important;}",
        }}
      />
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <header className="flex h-16 shrink-0 items-center gap-2 border-b">
            <div className="flex items-center gap-2 px-4 w-full h-full">
              <SidebarTrigger className="-ml-1" />
              <div data-tauri-drag-region className="w-full h-full flex justify-end items-center">
                {/* Window minimize button */}
                <Button variant="ghost" size="icon" onClick={handleMinimize}>
                  <Minus className="!h-5 !w-5" />
                </Button>
                {/* Window close button */}
                <Button variant="ghost" size="icon" onClick={handleClose}>
                  <X className="!h-5 !w-5" />
                </Button>
              </div>
            </div>
          </header>
          <main className="flex flex-1 flex-col p-4 overflow-y-auto min-h-0 max-h-[calc(800px-86px)]">
            {children}
          </main>
          <Toaster richColors />
        </SidebarInset>
      </SidebarProvider>
  </div>
  );
}
