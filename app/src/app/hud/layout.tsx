'use client';

import { AppProvider } from '@/lib/providers';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useRoleAccess } from '@/lib/role-access/useRoleAccess';

export default function HudLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Use role access
  useRoleAccess('hud');
  return (
    <AppProvider>
      <TooltipProvider>
        <>
          {/* Force transparent background for this window on first paint */}
          <style
            // eslint-disable-next-line react/no-danger
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