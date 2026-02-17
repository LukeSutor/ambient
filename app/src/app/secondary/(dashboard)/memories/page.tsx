"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
} from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { MemoryEntry } from "@/types/memory";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { Brain, ExternalLink, Sparkles, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

type MemoryListItem = {
  id: string;
  message_id: string;
  conversation_id: string | null;
  memory_type: string;
  text: string;
  timestamp: string;
  message_content: string | null;
};

const PAGE_SIZE = 20;

export default function MemoriesPage() {
  const [items, setItems] = useState<MemoryListItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const loaderRef = useRef<HTMLDivElement | null>(null);
  const didInitRef = useRef(false);
  const serverCountRef = useRef(0); // number of items fetched from the backend (excludes optimistic/event inserts)

  const loadPage = useCallback(async () => {
    if (isLoading || !hasMore) return;
    setIsLoading(true);
    setError(null);
    try {
      const offset = serverCountRef.current;
      const result = await invoke<MemoryListItem[]>(
        "get_memory_entries_with_message",
        {
          offset,
          limit: PAGE_SIZE,
        },
      );
      const page = result;
      // Deduplicate by id in case effects fire twice in dev or observer overlaps
      setItems((prev) => {
        const prevIds = new Set(prev.map((i) => i.id));
        const filtered = page.filter((i) => !prevIds.has(i.id));
        return [...prev, ...filtered];
      });
      serverCountRef.current += page.length;
      if (page.length < PAGE_SIZE) setHasMore(false);
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to load memories";
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [isLoading, hasMore]);

  // initial load
  useEffect(() => {
    if (didInitRef.current) return;
    didInitRef.current = true; // guard against React StrictMode double-invoke
    void loadPage();
  }, [loadPage]);

  // infinite scroll observer
  useEffect(() => {
    const node = loaderRef.current;
    if (!node) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const first = entries[0];
        if (first.isIntersecting) {
          void loadPage();
        }
      },
      { rootMargin: "600px" },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [loadPage]);

  // listen for memory_extracted events and prepend new item
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let mounted = true;
    void (async () => {
      try {
        unlisten = await listen<{ memory: MemoryEntry; timestamp: string }>(
          "memory_extracted",
          (e) => {
            if (!mounted) return;
            const mem = e.payload.memory;
            void (async () => {
              let messageContent: string | null = null;
              let conversationId: string | null = null;
              if (mem.message_id) {
                try {
                  const msg = await invoke<{
                    content?: string;
                    conversation_id?: string;
                  }>("get_message", {
                    messageId: mem.message_id,
                  });
                  messageContent = msg.content ?? "";
                  conversationId = msg.conversation_id ?? null;
                } catch (err) {
                  // If fetching fails, proceed without message content
                  console.warn("get_message failed for", mem.message_id, err);
                }
              }

              const newItem: MemoryListItem = {
                id: mem.id,
                message_id: mem.message_id,
                conversation_id: conversationId,
                memory_type: mem.memory_type,
                text: mem.text,
                timestamp: mem.timestamp,
                message_content: messageContent,
              };
              setItems((prev) => {
                if (prev.some((i) => i.id === newItem.id)) return prev;
                return [newItem, ...prev];
              });
            })();
            // Do not change serverCountRef here; it's only for backend-fetched items
          },
        );
      } catch (_) {
        // no-op: if listener fails, page still works
      }
    })();
    return () => {
      mounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  const onDeleteOne = useCallback(async (id: string) => {
    try {
      await invoke("delete_memory_entry", { id });
      setItems((prev) => prev.filter((m) => m.id !== id));
      toast.success("Memory deleted");
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to delete memory";
      toast.error(message);
    }
  }, []);

  const onDeleteAll = useCallback(async () => {
    try {
      await invoke("delete_all_memories");
      setItems([]);
      setHasMore(false);
      serverCountRef.current = 0;
      toast.success("All memories deleted");
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to delete all memories";
      toast.error(message);
    }
  }, []);

  const onOpenConversation = useCallback(async (item: MemoryListItem) => {
    if (!item.conversation_id) return;
    try {
      await invoke("open_main_window_at_conversation", {
        conversationId: item.conversation_id,
        messageId: item.message_id,
      });
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to open conversation";
      toast.error(message);
    }
  }, []);

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full max-w-4xl mx-auto">
      <div className="flex items-center justify-between w-full mb-6">
        <div>
          <h1 className="text-3xl font-bold font-sora">Memories</h1>
          <p className="text-muted-foreground mt-1">
            Facts and preferences learned from your conversations
          </p>
        </div>
        {items.length > 0 && (
          <Dialog>
            <DialogTrigger asChild>
              <Button variant="destructive" size="sm">
                <Trash2 className="mr-2 h-4 w-4" />
                Delete All
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Delete all memories?</DialogTitle>
                <DialogDescription>
                  This will permanently remove all memories and their
                  indexes.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose asChild>
                  <Button variant="secondary">Cancel</Button>
                </DialogClose>
                <DialogClose asChild>
                  <Button
                    variant="destructive"
                    onClick={() => {
                      void onDeleteAll();
                    }}
                  >
                    Delete all
                  </Button>
                </DialogClose>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        )}
      </div>

      {error && <div className="mb-3 text-sm text-red-600 w-full">{error}</div>}

      {items.length === 0 && !isLoading ? (
        <Empty className="border rounded-lg py-12 w-full">
          <EmptyMedia variant="icon">
            <Brain className="h-6 w-6" />
          </EmptyMedia>
          <EmptyHeader>
            <EmptyTitle>No memories yet</EmptyTitle>
            <EmptyDescription>
              Memories are automatically extracted from your conversations
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="flex flex-col gap-3 w-full">
          {items.map((m) => (
            <MemoryCard
              key={m.id}
              item={m}
              onDelete={() => {
                void onDeleteOne(m.id);
              }}
              onOpenConversation={() => {
                void onOpenConversation(m);
              }}
            />
          ))}
        </div>
      )}

      {/* loader sentinel */}
      {hasMore && <div ref={loaderRef} className="h-8" />}
      {isLoading && (
        <div className="mt-4 flex justify-center">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <div className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
            Loading...
          </div>
        </div>
      )}
    </div>
  );
}

function MemoryCard({
  item,
  onDelete,
  onOpenConversation,
}: {
  item: MemoryListItem;
  onDelete: () => void;
  onOpenConversation: () => void;
}) {
  const formattedDate = useMemo(() => {
    return new Date(item.timestamp).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }, [item.timestamp]);

  return (
    <Card className="hover:shadow-sm transition-shadow">
      <CardContent className="py-4 px-5">
        {/* Header: icon + text + actions */}
        <div className="flex items-start gap-3">
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary mt-0.5">
            <Sparkles className="h-4 w-4" />
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium leading-snug">{item.text}</p>
            {item.message_content && (
              <p className="mt-1.5 text-xs text-muted-foreground line-clamp-2">
                {item.message_content}
              </p>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-1">
            {item.conversation_id && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={onOpenConversation}
                  >
                    <ExternalLink className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Open conversation</TooltipContent>
              </Tooltip>
            )}
            <Dialog>
              <DialogTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 text-muted-foreground hover:text-destructive"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Delete this memory?</DialogTitle>
                  <DialogDescription>
                    This action cannot be undone.
                  </DialogDescription>
                </DialogHeader>
                <DialogFooter>
                  <DialogClose asChild>
                    <Button variant="secondary">Cancel</Button>
                  </DialogClose>
                  <DialogClose asChild>
                    <Button variant="destructive" onClick={onDelete}>
                      Delete
                    </Button>
                  </DialogClose>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </div>
        </div>
        {/* Footer: date */}
        <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
          <span>{formattedDate}</span>
        </div>
      </CardContent>
    </Card>
  );
}
