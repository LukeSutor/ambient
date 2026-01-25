"use client";

import AutoResizeContainer from "@/components/hud/auto-resize-container";
import { SiteHeader } from "@/components/site-header";
import { Toaster } from "@/components/ui/sonner";

export default function AuthLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <AutoResizeContainer
      widthType="login"
    >
      <div className="relative w-full h-full flex flex-col rounded-xl overflow-hidden">
        <SiteHeader />
        {children}
        <Toaster richColors position="top-center" />
      </div>
    </AutoResizeContainer>
  );
}
