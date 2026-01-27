import { cn } from "@/lib/utils";
import type React from "react";

interface ContentContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

export function ContentContainer({
  children,
  className,
  ...props
}: ContentContainerProps) {
  return (
    <div
      className={cn(
        "h-full text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-md overflow-hidden",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
