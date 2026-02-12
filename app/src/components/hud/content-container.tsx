import { cn } from "@/lib/utils";
import type React from "react";

interface ContentContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  isStreaming?: boolean;
}

export function ContentContainer({
  children,
  className,
  isStreaming,
  ...props
}: ContentContainerProps) {
  return (
    <div
      className={cn(
        "h-full text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-md overflow-hidden streaming-ring",
        isStreaming && "streaming-active border-transparent",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
