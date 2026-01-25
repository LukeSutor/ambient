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
    <div className="relative h-full max-h-[800px] w-full rounded-lg overflow-hidden border">
      <SiteHeader includeMinimize />
      {children}
      <Toaster richColors position="top-center" />
    </div>
  );
}
