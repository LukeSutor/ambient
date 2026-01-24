"use client";

import { Toaster } from "@/components/ui/sonner";
import type * as React from "react";

export default function SetupLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="relative h-full max-h-[800px] w-full rounded-lg overflow-hidden border">
      {children}
      <Toaster richColors position="top-center" />
    </div>
  );
}
