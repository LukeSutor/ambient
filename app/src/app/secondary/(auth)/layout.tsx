"use client";

import { useRoleAccess } from "@/lib/role-access";
import { Toaster } from "@/components/ui/sonner";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Minus, X } from "lucide-react";

export default function AuthLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Use role access
  useRoleAccess('/secondary');

  const handleClose = async () => {
    await invoke("close_secondary_window");
  }

  const handleMinimize = async () => {
    await invoke("minimize_secondary_window");
  }

  return (
    <div className="relative h-full max-h-[800px] w-full rounded-xl overflow-hidden border bg-gray-50 flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
      {/* Force transparent background for this window on first paint */}
      <style
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{
          __html:
            "html,body{background:transparent!important;background-color:transparent!important;}",
        }}
      />
      {/* Minimize, close, and drag region */}
      <div data-tauri-drag-region className="fixed top-0 left-0 right-0 border-b flex justify-end items-center pr-1 py-1">
        {/* Window minimize button */}
        <Button variant="ghost" size="icon" onClick={handleMinimize}>
            <Minus className="!h-5 !w-5" />
        </Button>
        {/* Window close button */}
        <Button variant="ghost" size="icon" onClick={handleClose}>
            <X className="!h-5 !w-5" />
        </Button>
      </div>
      <div className="max-w-md bg-black w-full space-y-8">
        {children}
      </div>
      <Toaster richColors position="top-center" />
    </div>
  );
}