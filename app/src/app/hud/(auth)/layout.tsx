"use client";

import AutoResizeContainer from "@/components/hud/auto-resize-container";
import { SiteHeader } from "@/components/site-header";
import { Toaster } from "@/components/ui/sonner";
import { useSettings } from "@/lib/settings/useSettings";
import type { HudDimensions } from "@/types/settings";
import { useEffect, useState } from "react";

export default function AuthLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Settings state
  const { getHudDimensions } = useSettings();
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(
    null,
  );

  useEffect(() => {
    void (async () => {
      const dimensions = await getHudDimensions();
      setHudDimensions(dimensions);
    })();
  }, [getHudDimensions]);

  return (
    <AutoResizeContainer
      hudDimensions={hudDimensions}
      widthType="login"
      className="bg-transparent"
    >
      <div className="relative w-full h-full flex flex-col">
        <SiteHeader includeMinimize />
        {children}
        <Toaster richColors position="top-center" />
      </div>
    </AutoResizeContainer>
  );
}
