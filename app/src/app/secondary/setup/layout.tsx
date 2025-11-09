"use client";

import * as React from "react";
import { Toaster } from "@/components/ui/sonner";

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
