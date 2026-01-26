"use client";

import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import { useConversation } from "@/lib/conversations";
import { AttachmentPreview } from "./attachment-preview";

export function AttachmentList() {
  const { attachmentData, removeAttachmentData, ocrLoading } = useConversation();

  if (attachmentData.length === 0 && !ocrLoading) return null;

  return (
    <ScrollArea className="flex justify-start items-center w-full space-x-2 py-1 px-3">
      <div className="flex w-max space-x-2 py-1">
        {attachmentData.map((attachment, index) => (
          <AttachmentPreview
            attachment={attachment}
            index={index}
            removeAttachmentData={removeAttachmentData}
            key={`${attachment.name}-${index}`}
          />
        ))}
        {ocrLoading && (
          <AttachmentPreview
            key="ocr-loading"
            isLoading={true}
            index={-1}
            removeAttachmentData={() => {}}
          />
        )}
      </div>
      <ScrollBar
        orientation="horizontal"
        className="[&_[data-slot='scroll-area-thumb']]:bg-black/25 [&_[data-slot='scroll-area-thumb']]:hover:bg-black/30"
      />
    </ScrollArea>
  );
}
