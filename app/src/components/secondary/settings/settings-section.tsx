"use client";

import type { ReactNode } from "react";

interface SettingsSectionProps {
  title: string;
  children: ReactNode;
  variant?: "default" | "danger";
}

export function SettingsSection({
  title,
  children,
  variant = "default",
}: SettingsSectionProps) {
  const borderClass =
    variant === "danger" ? "outline-red-500" : "outline-gray-300";

  return (
    <>
      <p className="text-xl font-semibold w-full pb-2">{title}</p>
      <div className={`outline ${borderClass} w-full rounded-md mb-6`}>
        {children}
      </div>
    </>
  );
}
