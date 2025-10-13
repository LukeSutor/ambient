'use client';

import { ConversationProvider } from '@/lib/conversations';
import { TooltipProvider } from '@/components/ui/tooltip';

export default function HudLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Nested layouts must not render <html> or <body>; keep this a client component now.
  return (
    <ConversationProvider>
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
    </ConversationProvider>
  );
}