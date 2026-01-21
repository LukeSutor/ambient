import { AttachmentData } from '@/types/events';
import { Button } from '../ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { Paperclip, X } from 'lucide-react';
import Image from 'next/image';

type AttachmentPreviewProps = {
  attachment: AttachmentData;
  index: number;
  removeAttachmentData: (index: number) => void;
};

export function AttachmentPreview({
  attachment, index,
  removeAttachmentData,}: AttachmentPreviewProps) {
  return (
    <div
      key={index}
      className="group relative h-20 flex flex-col items-center shrink-0 overflow-hidden"
    >
      <Tooltip>
        <TooltipTrigger asChild>
          <div className="relative h-full w-full">
            {attachment.file_type.startsWith('image/') ? (
              <img src={attachment.data} alt={attachment.name} className="h-20 w-20 object-cover rounded-lg" />
            ) :
            attachment.file_type === 'application/pdf' ? (
              <div className="flex flex-col justify-center items-center space-y-2 w-full h-full px-4 bg-white/20 border border-black/20 rounded-lg">
                <p className="font-semibold truncate max-w-32 mr-8">{attachment.name}</p>
                <div className="flex flex-row justify-start items-center space-x-2 w-full">
                  <Image src='/pdf-icon.png' alt='PDF Icon' width={16} height={16} />
                  <p className="text-sm">PDF</p>
                </div>
              </div>
            ) : null}
            <Button
              variant="ghost"
              className="hidden group-hover:flex absolute top-1.5 right-1.5 h-8 w-8 p-0 rounded-full text-black bg-white/60 hover:bg-white/80 border border-black/10 shadow-sm"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                removeAttachmentData(index);
              }}
            >
              <X className="!h-6 !w-6 text-black shrink-0" />
            </Button>
          </div>
        </TooltipTrigger>
        <TooltipContent>{attachment.name}</TooltipContent>
      </Tooltip>
    </div>
  );
}