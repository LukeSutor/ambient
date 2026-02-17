"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { AutomationNotification } from "@/types/automations";
import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, X, XCircle } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

export function AutomationNotificationBanner() {
  const [notification, setNotification] =
    useState<AutomationNotification | null>(null);
  const [visible, setVisible] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const [contentHeight, setContentHeight] = useState(0);

  useEffect(() => {
    const setup = async () => {
      const unlisten = await listen<AutomationNotification>(
        "automation_notification",
        (event) => {
          setNotification(event.payload);
          setVisible(true);
        },
      );
      return unlisten;
    };

    const cleanup = setup();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, []);

  // Measure content height for smooth animation
  useEffect(() => {
    if (contentRef.current && visible) {
      setContentHeight(contentRef.current.scrollHeight);
    }
  }, [notification, visible]);

  const dismiss = useCallback(() => {
    setVisible(false);
    // Clear notification after animation completes
    setTimeout(() => setNotification(null), 300);
  }, []);

  if (!notification) return null;

  const isError = notification.notification_type === "error";

  return (
    <div
      className="overflow-hidden transition-all duration-300 ease-in-out"
      style={{
        maxHeight: visible ? `${contentHeight + 16}px` : "0px",
        opacity: visible ? 1 : 0,
      }}
    >
      <div
        ref={contentRef}
        className={`rounded-xl border p-3 mb-2 ${
          isError
            ? "bg-destructive/10 border-destructive/30"
            : "bg-primary/5 border-primary/20"
        }`}
      >
        <div className="flex items-start gap-2">
          {isError ? (
            <XCircle className="h-4 w-4 mt-0.5 text-destructive shrink-0" />
          ) : (
            <CheckCircle2 className="h-4 w-4 mt-0.5 text-primary shrink-0" />
          )}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-sm font-medium truncate">
                {notification.title}
              </span>
              <Badge
                variant={isError ? "destructive" : "default"}
                className="text-[10px] px-1.5 py-0"
              >
                {isError ? "Error" : "Done"}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground line-clamp-3">
              {notification.body}
            </p>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 shrink-0"
            onClick={dismiss}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
    </div>
  );
}
