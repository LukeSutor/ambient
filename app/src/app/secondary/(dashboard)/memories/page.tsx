"use client";

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
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
import type { MemoryEntry } from "@/types/memory";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type MemoryListItem = {
  id: string;
  message_id: string;
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
              if (mem.message_id) {
                try {
                  const msg = await invoke<{ content?: string }>(
                    "get_message",
                    {
                      messageId: mem.message_id,
                    },
                  );
                  if (!mounted) return;
                  messageContent = msg.content ?? "";
                } catch (err) {
                  // If fetching fails, proceed without message content
                  console.warn("get_message failed for", mem.message_id, err);
                }
              }

              if (!mounted) return;
              const newItem: MemoryListItem = {
                id: mem.id,
                message_id: mem.message_id,
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
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to delete memory";
      setError(message);
    }
  }, []);

  const onDeleteAll = useCallback(async () => {
    try {
      await invoke("delete_all_memories");
      setItems([]);
      setHasMore(false);
      serverCountRef.current = 0;
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to delete all memories";
      setError(message);
    }
  }, []);

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full">
      <div className="w-full max-w-3xl">
        <Card className="w-full">
          <CardHeader>
            <CardTitle>Memories</CardTitle>
          </CardHeader>
          <CardContent>
            {error && <div className="mb-3 text-sm text-red-600">{error}</div>}

            {items.length === 0 && !isLoading ? (
              <div className="text-sm text-muted-foreground">
                No memories yet.
              </div>
            ) : (
              <Accordion type="single" collapsible className="w-full">
                {items.map((m) => (
                  <AccordionItem key={m.id} value={m.id}>
                    <div className="flex flex-row justify-between items-center gap-4 w-full">
                      <div className="w-full">
                        <AccordionTrigger className="w-full text-left">
                          <span className="truncate font-medium">{m.text}</span>
                        </AccordionTrigger>
                      </div>
                      <Dialog>
                        <DialogTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label="Delete memory"
                            className="shrink-0"
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
                              <Button
                                variant="destructive"
                                onClick={() => onDeleteOne(m.id)}
                              >
                                Delete
                              </Button>
                            </DialogClose>
                          </DialogFooter>
                        </DialogContent>
                      </Dialog>
                    </div>
                    <AccordionContent>
                      <div className="flex flex-col gap-2 text-sm">
                        <div>
                          <div className="text-xs text-muted-foreground">
                            Original Message
                          </div>
                          <div className="whitespace-pre-wrap break-words">
                            {m.message_content}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs text-muted-foreground">
                            Saved Memory
                          </div>
                          <div className="whitespace-pre-wrap break-words">
                            {m.text}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs text-muted-foreground">
                            Saved On
                          </div>
                          <div className="whitespace-pre-wrap break-words">
                            {new Date(m.timestamp).toLocaleString()}
                          </div>
                        </div>
                      </div>
                    </AccordionContent>
                  </AccordionItem>
                ))}
              </Accordion>
            )}

            {/* loader sentinel */}
            {hasMore && <div ref={loaderRef} className="h-8" />}
            {isLoading && (
              <div className="mt-2 text-sm text-muted-foreground">Loading…</div>
            )}
          </CardContent>
          <CardFooter className="justify-end">
            <Dialog>
              <DialogTrigger asChild>
                <Button variant="destructive">Delete all memories</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Delete all memories?</DialogTitle>
                  <DialogDescription>
                    This will permanently remove all memories and their indexes.
                  </DialogDescription>
                </DialogHeader>
                <DialogFooter>
                  <DialogClose asChild>
                    <Button variant="secondary">Cancel</Button>
                  </DialogClose>
                  <DialogClose asChild>
                    <Button variant="destructive" onClick={onDeleteAll}>
                      Delete all
                    </Button>
                  </DialogClose>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}
