"use client";

import { TooltipProvider } from "@/components/ui/tooltip";
import { AppProvider } from "@/lib/providers";
import { useRoleAccess } from "@/lib/role-access";
import { useSettings } from "@/lib/settings";

export default function HudLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Use role access
  useRoleAccess("/hud");
  // Use root settings
  useSettings(true);
  return (
    <AppProvider>
      <TooltipProvider>
        <>
          {/* Force transparent background for this window on first paint */}
          <style
            /* biome-ignore lint/security/noDangerouslySetInnerHtml: Need to force transparent background for Tauri window */
            dangerouslySetInnerHTML={{
              __html:
                "html,body{background:transparent!important;background-color:transparent!important;}",
            }}
          />
          <div className="w-screen h-screen overflow-hidden bg-transparent antialiased font-sans">
            {children}
          </div>
        </>
      </TooltipProvider>
    </AppProvider>
  );
}
