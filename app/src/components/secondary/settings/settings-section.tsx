"use client";

import React from "react";
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

  const separatorClass = borderClass.replace("outline-", "border-");

  const childrenArray = React.Children.toArray(children);

  return (
    <>
      <p className="text-xl font-semibold w-full pb-2">{title}</p>
      <div className={`outline ${borderClass} w-full rounded-md mb-6`}>
        {childrenArray.map((child, index) => {
          const key =
            React.isValidElement(child) && child.key != null
              ? child.key
              : `section-${title}-child-${String(index)}`;
          return (
            <React.Fragment key={key}>
              {child}
              {index < childrenArray.length - 1 && (
                <div className={`border-t ${separatorClass}`} />
              )}
            </React.Fragment>
          );
        })}
      </div>
    </>
  );
}
