"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
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
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { invoke } from "@tauri-apps/api/core";
import {
  Camera,
  ExternalLink,
  FileText,
  FolderOpen,
  Image as ImageIcon,
  Search,
  SquareDashed,
  Trash2,
  X,
} from "lucide-react";
import Image from "next/image";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { AttachmentListItem } from "@/types/conversations";

const PAGE_SIZE = 20;

type AttachmentWithData = AttachmentListItem & {
  dataUrl?: string;
  isLoadingData?: boolean;
};

export default function UploadsPage() {
  const [items, setItems] = useState<AttachmentWithData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [previewItem, setPreviewItem] = useState<AttachmentWithData | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<AttachmentWithData | null>(null);

  const loaderRef = useRef<HTMLDivElement | null>(null);
  const didInitRef = useRef(false);
  const serverCountRef = useRef(0);

  // Debounce search input
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchTerm);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchTerm]);

  // Reset list when search changes
  useEffect(() => {
    setItems([]);
    serverCountRef.current = 0;
    setHasMore(true);
    didInitRef.current = false;
  }, [debouncedSearch]);

  const loadPage = useCallback(async () => {
    if (isLoading || !hasMore) return;
    setIsLoading(true);
    setError(null);
    try {
      const offset = serverCountRef.current;
      const result = await invoke<AttachmentListItem[]>("list_attachments", {
        limit: PAGE_SIZE,
        offset,
        search: debouncedSearch || null,
      });
      
      // Deduplicate by id
      setItems((prev) => {
        const prevIds = new Set(prev.map((i) => i.id));
        const filtered = result.filter((i) => !prevIds.has(i.id));
        return [...prev, ...filtered];
      });
      serverCountRef.current += result.length;
      if (result.length < PAGE_SIZE) setHasMore(false);
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to load uploads";
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, [isLoading, hasMore, debouncedSearch]);

  // Initial load
  useEffect(() => {
    if (didInitRef.current) return;
    didInitRef.current = true;
    void loadPage();
  }, [loadPage]);

  // Infinite scroll observer
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

  // Load attachment data for preview
  const loadAttachmentData = useCallback(async (item: AttachmentWithData) => {
    if (item.dataUrl || item.isLoadingData) return item;
    
    setItems((prev) =>
      prev.map((i) =>
        i.id === item.id ? { ...i, isLoadingData: true } : i
      )
    );

    try {
      const data = await invoke<string>("get_attachment_data", {
        attachmentId: item.id,
      });
      
      setItems((prev) =>
        prev.map((i) =>
          i.id === item.id ? { ...i, dataUrl: data, isLoadingData: false } : i
        )
      );
      
      return { ...item, dataUrl: data, isLoadingData: false };
    } catch (e) {
      console.error("Failed to load attachment data:", e);
      setItems((prev) =>
        prev.map((i) =>
          i.id === item.id ? { ...i, isLoadingData: false } : i
        )
      );
      return item;
    }
  }, []);

  // Handle opening preview
  const handleOpenPreview = useCallback(async (item: AttachmentWithData) => {
    const updatedItem = await loadAttachmentData(item);
    setPreviewItem(updatedItem);
  }, [loadAttachmentData]);

  // Delete handler
  const onDelete = useCallback(async (item: AttachmentWithData) => {
    try {
      await invoke("delete_attachment", { attachmentId: item.id });
      setItems((prev) => prev.filter((i) => i.id !== item.id));
      serverCountRef.current = Math.max(0, serverCountRef.current - 1);
      toast.success("Attachment deleted successfully");
      setDeleteTarget(null);
    } catch (e: unknown) {
      const message =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : "Failed to delete attachment";
      toast.error(message);
    }
  }, []);

  // Navigate to conversation
  const onOpenInConversation = useCallback(async (item: AttachmentWithData) => {
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

  // Get icon for attachment type
  const getAttachmentIcon = useCallback((fileType: string) => {
    if (fileType.startsWith("image/")) {
      return <ImageIcon className="h-5 w-5 text-emerald-600" />;
    }
    if (fileType === "application/pdf") {
      return (
        <Image src="/pdf-icon.png" alt="PDF" width={20} height={20} />
      );
    }
    return <SquareDashed className="h-5 w-5 text-blue-600" />;
  }, []);

  // Get type label
  const getTypeLabel = useCallback((fileType: string) => {
    if (fileType.startsWith("image/")) return "Image";
    if (fileType === "application/pdf") return "PDF";
    if (fileType === "ambient/ocr") return "OCR";
    return "File";
  }, []);

  // Get type badge color
  const getTypeBadgeColor = useCallback((fileType: string) => {
    if (fileType.startsWith("image/"))
      return "bg-emerald-500/10 text-emerald-600";
    if (fileType === "application/pdf") return "bg-red-500/10 text-red-600";
    if (fileType === "ambient/ocr") return "bg-blue-500/10 text-blue-600";
    return "bg-gray-500/10 text-gray-600";
  }, []);

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full">
      <div className="w-full max-w-4xl">
        <Card className="w-full">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>Your Uploads</CardTitle>
                <CardDescription>
                  Images, PDFs, and OCR captures from your conversations
                </CardDescription>
              </div>
            </div>
            <div className="relative mt-4">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search by file name..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="pl-9"
              />
              {searchTerm && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7"
                  onClick={() => setSearchTerm("")}
                >
                  <X className="h-4 w-4" />
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent>
            {error && (
              <div className="mb-3 text-sm text-red-600">{error}</div>
            )}

            {items.length === 0 && !isLoading ? (
              <Empty className="border rounded-lg py-12">
                <EmptyMedia variant="icon">
                  <FolderOpen className="h-6 w-6" />
                </EmptyMedia>
                <EmptyHeader>
                  <EmptyTitle>
                    {debouncedSearch ? "No results found" : "No uploads yet"}
                  </EmptyTitle>
                  <EmptyDescription>
                    {debouncedSearch
                      ? "Try a different search term"
                      : "Upload images, PDFs, or capture screen content in your conversations"}
                  </EmptyDescription>
                </EmptyHeader>
              </Empty>
            ) : (
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                {items.map((item) => (
                  <UploadCard
                    key={item.id}
                    item={item}
                    onPreview={() => handleOpenPreview(item)}
                    onDelete={() => setDeleteTarget(item)}
                    onOpenConversation={() => onOpenInConversation(item)}
                    getAttachmentIcon={getAttachmentIcon}
                    getTypeLabel={getTypeLabel}
                    getTypeBadgeColor={getTypeBadgeColor}
                    loadAttachmentData={loadAttachmentData}
                  />
                ))}
              </div>
            )}

            {/* Loader sentinel */}
            {hasMore && <div ref={loaderRef} className="h-8" />}
            {isLoading && (
              <div className="mt-4 flex justify-center">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <div className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                  Loading...
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Preview Dialog */}
      <PreviewDialog
        item={previewItem}
        onClose={() => setPreviewItem(null)}
        onOpenConversation={onOpenInConversation}
        getAttachmentIcon={getAttachmentIcon}
      />

      {/* Delete Confirmation Dialog */}
      <Dialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete this upload?</DialogTitle>
            <DialogDescription>
              This will permanently delete "{deleteTarget?.file_name}" and remove it from the conversation.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button variant="secondary">Cancel</Button>
            </DialogClose>
            <Button
              variant="destructive"
              onClick={() => deleteTarget && onDelete(deleteTarget)}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// Upload Card Component
function UploadCard({
  item,
  onPreview,
  onDelete,
  onOpenConversation,
  getAttachmentIcon,
  getTypeLabel,
  getTypeBadgeColor,
  loadAttachmentData,
}: {
  item: AttachmentWithData;
  onPreview: () => void;
  onDelete: () => void;
  onOpenConversation: () => void;
  getAttachmentIcon: (fileType: string) => React.ReactNode;
  getTypeLabel: (fileType: string) => string;
  getTypeBadgeColor: (fileType: string) => string;
  loadAttachmentData: (item: AttachmentWithData) => Promise<AttachmentWithData>;
}) {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [isLoadingThumbnail, setIsLoadingThumbnail] = useState(false);

  // Load thumbnail for images
  useEffect(() => {
    if (!item.file_type.startsWith("image/") || item.dataUrl) {
      if (item.dataUrl) setThumbnailUrl(item.dataUrl);
      return;
    }

    const loadThumbnail = async () => {
      setIsLoadingThumbnail(true);
      try {
        const updated = await loadAttachmentData(item);
        if (updated.dataUrl) {
          setThumbnailUrl(updated.dataUrl);
        }
      } finally {
        setIsLoadingThumbnail(false);
      }
    };

    void loadThumbnail();
  }, [item, loadAttachmentData]);

  const formattedDate = useMemo(() => {
    return new Date(item.created_at).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  }, [item.created_at]);

  return (
    <div className="group relative flex flex-col rounded-lg border bg-card overflow-hidden hover:border-primary/50 transition-colors">
      {/* Preview area */}
      <button
        type="button"
        onClick={onPreview}
        className="relative aspect-video w-full bg-muted/30 flex items-center justify-center overflow-hidden"
      >
        {item.file_type.startsWith("image/") ? (
          isLoadingThumbnail ? (
            <div className="h-8 w-8 animate-pulse bg-muted rounded" />
          ) : thumbnailUrl ? (
            <img
              src={thumbnailUrl}
              alt={item.file_name}
              className="w-full h-full object-cover group-hover:scale-105 transition-transform"
            />
          ) : (
            <ImageIcon className="h-8 w-8 text-muted-foreground" />
          )
        ) : item.file_type === "application/pdf" ? (
          <div className="flex flex-col items-center gap-2">
            <Image src="/pdf-icon.png" alt="PDF" width={40} height={40} />
            <span className="text-xs text-muted-foreground">PDF Document</span>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-2 px-4">
            <SquareDashed className="h-8 w-8 text-blue-600" />
            <p className="text-xs text-muted-foreground line-clamp-3 text-center">
              {item.extracted_text?.substring(0, 100) || "OCR Capture"}
              {item.extracted_text && item.extracted_text.length > 100 && "..."}
            </p>
          </div>
        )}
        <div className="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-colors flex items-center justify-center">
          <div className="opacity-0 group-hover:opacity-100 transition-opacity">
            <Search className="h-6 w-6 text-white drop-shadow-lg" />
          </div>
        </div>
      </button>

      {/* Info area */}
      <div className="flex flex-col gap-2 p-3">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <Tooltip>
              <TooltipTrigger asChild>
                <p className="text-sm font-medium truncate">
                  {item.file_name}
                </p>
              </TooltipTrigger>
              <TooltipContent>{item.file_name}</TooltipContent>
            </Tooltip>
            <p className="text-xs text-muted-foreground mt-0.5">
              {formattedDate}
            </p>
          </div>
          <span
            className={`text-[10px] px-1.5 py-0.5 rounded font-bold uppercase shrink-0 ${getTypeBadgeColor(item.file_type)}`}
          >
            {getTypeLabel(item.file_type)}
          </span>
        </div>

        <Tooltip>
          <TooltipTrigger asChild>
            <p className="text-xs text-muted-foreground truncate">
              From: {item.conversation_name}
            </p>
          </TooltipTrigger>
          <TooltipContent>{item.conversation_name}</TooltipContent>
        </Tooltip>

        {/* Actions */}
        <div className="flex items-center gap-1 pt-1 border-t mt-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 flex-1 text-xs"
                onClick={onOpenConversation}
              >
                <ExternalLink className="h-3.5 w-3.5 mr-1" />
                Open Chat
              </Button>
            </TooltipTrigger>
            <TooltipContent>Open this attachment in its conversation</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-destructive hover:text-destructive hover:bg-destructive/10"
                onClick={onDelete}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Delete this upload</TooltipContent>
          </Tooltip>
        </div>
      </div>
    </div>
  );
}

// Preview Dialog Component
function PreviewDialog({
  item,
  onClose,
  onOpenConversation,
  getAttachmentIcon,
}: {
  item: AttachmentWithData | null;
  onClose: () => void;
  onOpenConversation: (item: AttachmentWithData) => void;
  getAttachmentIcon: (fileType: string) => React.ReactNode;
}) {
  if (!item) return null;

  return (
    <Dialog open={!!item} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[90vw] max-h-[90vh] p-0 overflow-hidden border-none shadow-2xl bg-zinc-100 flex flex-col gap-0">
        <DialogDescription className="sr-only">
          Preview of {item.file_name}
        </DialogDescription>
        <DialogHeader className="shrink-0 p-4 border-b bg-white flex flex-row items-center justify-between space-y-0">
          <DialogTitle className="text-sm truncate font-bold flex items-center gap-2 pr-8">
            {getAttachmentIcon(item.file_type)}
            {item.file_name}
          </DialogTitle>
        </DialogHeader>
        <div className="flex-1 w-full p-4 flex items-center justify-center bg-zinc-100/50 min-h-0 overflow-auto">
          {item.file_type.startsWith("image/") && item.dataUrl ? (
            <img
              src={item.dataUrl}
              alt={item.file_name}
              className="max-w-full max-h-[70vh] object-contain rounded-lg shadow-lg"
            />
          ) : item.file_type === "application/pdf" && item.dataUrl ? (
            <iframe
              title="PDF Preview"
              src={item.dataUrl}
              className="w-full h-[70vh] bg-white rounded-lg border shadow-inner"
            />
          ) : (
            <div className="w-full max-w-2xl bg-white p-8 rounded-lg border shadow-sm max-h-[70vh] overflow-y-auto">
              <pre className="text-sm leading-relaxed text-black/70 font-mono whitespace-pre-wrap">
                {item.dataUrl || item.extracted_text || "No content available"}
              </pre>
            </div>
          )}
        </div>
        <div className="shrink-0 p-4 border-t bg-white flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            From conversation: <span className="font-medium text-foreground">{item.conversation_name}</span>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => onOpenConversation(item)}
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              Open in Chat
            </Button>
            <DialogClose asChild>
              <Button variant="secondary" size="sm">
                Close
              </Button>
            </DialogClose>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
