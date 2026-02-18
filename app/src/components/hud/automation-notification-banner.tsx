"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { AutomationNotification } from "@/types/automations";
import { listen } from "@tauri-apps/api/event";
import {
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  X,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { ContentContainer } from "./content-container";

export function AutomationNotificationBanner() {
  const [queue, setQueue] = useState<AutomationNotification[]>([]);
  const [index, setIndex] = useState(0);
  const [sliding, setSliding] = useState<"left" | "right" | null>(null);

  useEffect(() => {
    const setup = async () => {
      const unlisten = await listen<AutomationNotification>(
        "automation_notification",
        (event) => {
          setQueue((prev) => {
            const next = [...prev, event.payload];
            // auto-advance to newest notification
            setIndex(next.length - 1);
            return next;
          });
        },
      );
      return unlisten;
    };
    const cleanup = setup();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, []);

  const dismiss = useCallback((idx: number) => {
    setQueue((prev) => {
      const next = prev.filter((_, i) => i !== idx);
      setIndex((prevIdx) => Math.min(prevIdx, Math.max(0, next.length - 1)));
      return next;
    });
  }, []);

  const navigate = useCallback(
    (dir: "prev" | "next") => {
      if (sliding) return;
      const dir2 = dir === "next" ? "left" : "right";
      setSliding(dir2);
      setTimeout(() => {
        setIndex((prev) => (dir === "next" ? prev + 1 : prev - 1));
        setSliding(null);
      }, 180);
    },
    [sliding],
  );

  if (queue.length === 0) return null;

  const notification = queue[index];
  if (!notification) return null;

  const isError = notification.notification_type === "error";

  return (
    <ContentContainer className="mb-2">
      <div className="p-3 overflow-hidden">
        <div
          className="flex items-start gap-2"
          style={{
            transform:
              sliding === "left"
                ? "translateX(-12px)"
                : sliding === "right"
                  ? "translateX(12px)"
                  : "translateX(0)",
            opacity: sliding ? 0 : 1,
            transition: "transform 0.18s ease, opacity 0.18s ease",
          }}
        >
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
              {isError && (
                <Badge
                  variant="destructive"
                  className="text-[10px] px-1.5 py-0"
                >
                  Error
                </Badge>
              )}
              {queue.length > 1 && (
                <span className="text-xs text-muted-foreground ml-auto shrink-0">
                  {index + 1}/{queue.length}
                </span>
              )}
            </div>
            <p className="text-xs text-muted-foreground line-clamp-3">
              {notification.body}
            </p>
          </div>

          <div className="flex items-center gap-0.5 shrink-0">
            {queue.length > 1 && (
              <>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => navigate("prev")}
                  disabled={index === 0 || !!sliding}
                >
                  <ChevronLeft className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6"
                  onClick={() => navigate("next")}
                  disabled={index === queue.length - 1 || !!sliding}
                >
                  <ChevronRight className="h-3.5 w-3.5" />
                </Button>
              </>
            )}
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6"
              onClick={() => dismiss(index)}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>
    </ContentContainer>
  );
}
