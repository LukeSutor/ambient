"use client";

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
  return <div className="w-screen h-screen overflow-hidden">{children}</div>;
}
