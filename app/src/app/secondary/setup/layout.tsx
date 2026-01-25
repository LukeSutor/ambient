"use client";

import { SiteHeader } from "@/components/site-header";
import { Toaster } from "@/components/ui/sonner";
import type * as React from "react";

export default function SetupLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="relative bg-background h-screen w-full rounded-lg overflow-hidden border flex flex-col pt-16">
      <SiteHeader includeMinimize />
      <div className="flex-1 overflow-auto">
        {children}
      </div>
      <Toaster richColors position="top-center" />
    </div>
  );
}
