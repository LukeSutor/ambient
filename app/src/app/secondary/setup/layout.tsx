"use client";

import { Toaster } from "@/components/ui/sonner";
import type * as React from "react";

export default function SetupLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="min-h-screen bg-gray-50">
      {children}
      <Toaster richColors position="top-center" />
    </div>
  );
}
