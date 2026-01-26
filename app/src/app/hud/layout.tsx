"use client";

import { useRoleAccess } from "@/lib/role-access";

export default function HudLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Use role access
  useRoleAccess("/hud");
  return <div className="w-screen h-screen overflow-hidden">{children}</div>;
}
