import { AttachmentData } from '@/types/events';
import { Button } from '../ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import { SquareDashed, X, Search, Eye } from 'lucide-react';
import Image from 'next/image';
import { useState } from 'react';
import { useWindows } from '@/lib/windows/useWindows';

type AttachmentPreviewProps = {
  attachment: AttachmentData;
  index: number;
  removeAttachmentData: (index: number) => void;
};

export function AttachmentPreview({
  attachment, index,
  removeAttachmentData,}: AttachmentPreviewProps) {
  const [preview, setPreview] = useState(false);
  const { isChatExpanded } = useWindows();
  return (
    <div
      key={index}
      className={`group relative h-20 flex flex-col items-center shrink-0 ${(preview && !isChatExpanded) ? "mb-[300px]" : ""}`}
    >
      <Dialog open={preview} onOpenChange={setPreview}>
        <Tooltip>
          <TooltipTrigger asChild>
            <DialogTrigger asChild>
              <button className="relative h-full w-full outline-none">
                {attachment.file_type.startsWith('image/') ? (
                  <div className="relative h-20 w-20 group/img">
                    <img 
                      src={attachment.data} 
                      alt={attachment.name} 
                      className="h-20 w-20 object-cover rounded-xl border border-black/10 shadow-sm transition-all group-hover/img:brightness-75" 
                    />
                    <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover/img:opacity-100 transition-opacity">
                      <Search className="w-5 h-5 text-white" />
                    </div>
                  </div>
                ) : attachment.file_type === 'application/pdf' ? (
                  <div className="flex items-center gap-3 h-20 w-56 px-3 bg-white/40 border border-black/10 rounded-xl hover:bg-white/60 transition-all group/file">
                    <div className="h-12 w-12 flex items-center justify-center bg-red-500/10 rounded-lg flex-shrink-0 group-hover/file:bg-red-500/20 transition-colors">
                      <Image src='/pdf-icon.png' alt='PDF Icon' width={24} height={24} />
                    </div>
                    <div className="flex-1 min-w-0 text-left">
                      <p className="text-xs font-bold truncate text-black/80">{attachment.name}</p>
                      <span className="text-[9px] bg-red-500/10 text-red-600 px-1.5 py-0.5 rounded font-bold uppercase mt-1 inline-block">PDF</span>
                    </div>
                    <Eye className="w-4 h-4 text-black/20 opacity-0 group-hover/file:opacity-100 transition-opacity" />
                  </div>
                ) : (
                  <div className="flex items-center gap-3 h-20 w-56 px-3 bg-white/40 border border-black/10 rounded-xl hover:bg-white/60 transition-all group/ocr">
                    <div className="h-12 w-12 flex items-center justify-center bg-blue-500/10 rounded-lg flex-shrink-0 group-hover/ocr:bg-blue-500/20 transition-colors">
                      <SquareDashed className="h-6 w-6 text-blue-600" />
                    </div>
                    <div className="flex-1 min-w-0 text-left">
                      <p className="text-xs font-bold truncate text-black/80">{attachment.name || 'Screen Capture'}</p>
                      <span className="text-[9px] bg-blue-500/10 text-blue-600 px-1.5 py-0.5 rounded font-bold uppercase mt-1 inline-block">OCR</span>
                    </div>
                    <Eye className="w-4 h-4 text-black/20 opacity-0 group-hover/ocr:opacity-100 transition-opacity" />
                  </div>
                )}
              </button>
            </DialogTrigger>
          </TooltipTrigger>
          <TooltipContent side="top">
            {attachment.file_type === 'ambient/ocr' ? 'Click to view text' : `Preview ${attachment.name}`}
          </TooltipContent>
        </Tooltip>

        <DialogContent className="sm:max-w-[90vw] h-[90vh] p-0 overflow-hidden border-none shadow-2xl bg-zinc-100 flex flex-col gap-0">
          <DialogHeader className="shrink-0 p-4 border-b bg-white flex flex-row items-center justify-between space-y-0">
            <DialogTitle className="text-sm truncate font-bold flex items-center gap-2 pr-8">
              {attachment.file_type.startsWith('image/') ? (
                <div className="h-4 w-4 bg-black/5 rounded flex items-center justify-center">
                  <div className="w-2 h-2 bg-black/40 rounded-sm" />
                </div>
              ) : attachment.file_type === 'application/pdf' ? (
                <Image src='/pdf-icon.png' alt='PDF' width={16} height={16} />
              ) : (
                <SquareDashed className="h-4 w-4 text-blue-600" />
              )}
              {attachment.name || 'Preview'}
            </DialogTitle>
          </DialogHeader>
          <div className="flex-1 w-full p-4 flex items-center justify-center bg-zinc-100/50 min-h-0">
            {attachment.file_type.startsWith('image/') ? (
              <img 
                src={attachment.data} 
                alt={attachment.name} 
                className="max-w-full max-h-full object-contain rounded-lg shadow-lg" 
              />
            ) : attachment.file_type === 'application/pdf' ? (
              <iframe src={attachment.data} className="w-full h-full bg-white rounded-lg border shadow-inner" />
            ) : (
              <div className="w-full max-w-2xl bg-white p-8 rounded-xl border shadow-sm h-full overflow-y-auto">
                <pre className="text-sm leading-relaxed text-black/70 font-mono whitespace-pre-wrap">
                  {attachment.data}
                </pre>
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>

      <Button
        variant="ghost"
        className="absolute -top-1 -right-1 h-6 w-6 p-0 rounded-full text-white bg-black/80 hover:bg-black border border-white/20 shadow-lg z-20 flex"
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          removeAttachmentData(index);
        }}
      >
        <X className="h-3.5 w-3.5 shrink-0" />
      </Button>
    </div>
  );
}