import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Conversation } from '@/types/conversations';
import { useConversation } from '@/lib/conversations';
import { HudDimensions } from '@/types/settings';
import { useWindows } from '@/lib/windows/useWindows';
import { ContentContainer } from './content-container';
import { Loader2, Ellipsis, Trash2, Pen, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";

interface ConversationListProps {
  hudDimensions: HudDimensions | null;
}

const CONVERSATION_LIMIT = 20;
const SKELETON_COUNT = 3;

export function ConversationList({
  hudDimensions
}: ConversationListProps) {
  // State
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [loadingConversations, setLoadingConversations] = useState<boolean>(true);
  const [loadingMore, setLoadingMore] = useState<boolean>(false);
  const [conversationPage, setConversationPage] = useState<number>(0);
  const [hasMoreConversations, setHasMoreConversations] = useState<boolean>(true);
  const [newConversationName, setNewConversationName] = useState<string>('');
  const [editingConversationId, setEditingConversationId] = useState<string | null>(null);

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const observerTarget = useRef<HTMLDivElement>(null);

  // Conversation Manager
  const {
    messages,
    getConversations,
  } = useConversation(messagesEndRef);

  // Window Manager
  const {
    isChatExpanded,
    isChatHistoryExpanded,
    toggleChatHistory,
  } = useWindows();

  // Set editing conversation ID to null when escape key is pressed
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setEditingConversationId(null);
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // Load conversations on mount
  useEffect(() => {
    let isMounted = true;

    const loadConversations = async () => {
      setLoadingConversations(true);
      const convs = await getConversations(CONVERSATION_LIMIT, 0);
      if (isMounted) {
        setConversations(convs);
        setHasMoreConversations(convs.length === CONVERSATION_LIMIT);
        setConversationPage(0);
      }
      setLoadingConversations(false);
    };

    loadConversations();

    return () => {
      isMounted = false;
    };
  }, [getConversations]);

  // Load more conversations function
  const loadMoreConversations = useCallback(async () => {
    if (loadingMore || !hasMoreConversations) return;

    setLoadingMore(true);
    const nextPage = conversationPage + 1;
    const newConvs = await getConversations(CONVERSATION_LIMIT, nextPage * CONVERSATION_LIMIT);
    
    if (newConvs.length > 0) {
      setConversations(prev => [...prev, ...newConvs]);
      setConversationPage(nextPage);
      setHasMoreConversations(newConvs.length === CONVERSATION_LIMIT);
    } else {
      setHasMoreConversations(false);
    }
    
    setLoadingMore(false);
  }, [conversationPage, loadingMore, hasMoreConversations, getConversations]);

  // // Intersection Observer for infinite scroll
  // useEffect(() => {
  //   const observer = new IntersectionObserver(
  //     (entries) => {
  //       if (entries[0].isIntersecting && hasMoreConversations && !loadingMore) {
  //         loadMoreConversations();
  //       }
  //     },
  //     { threshold: 0.1 }
  //   );

  //   const currentTarget = observerTarget.current;
  //   if (currentTarget) {
  //     console.log("observing")
  //     observer.observe(currentTarget);
  //   } else {
  //     console.log("not observing")
  //   }

  //   return () => {
  //     if (currentTarget) {
  //       observer.unobserve(currentTarget);
  //     }
  //   };
  // }, [hasMoreConversations, loadingMore, loadMoreConversations, isChatHistoryExpanded]);

  const handleUpdateConversationName = async () => {
    console.log("updating");
  };

  return (
    <ContentContainer>
      <div
        className="flex flex-col w-full h-full hud-scroll overflow-y-auto pt-4 px-2"
      >
        <div className="flex flex-row justify-between items-center mb-2 ml-3 text-black/80">
          <p className="text-sm font-semibold whitespace-nowrap">Chat History</p>
          <Button onClick={() => toggleChatHistory(false)} variant="ghost" size="icon" className="!p-2">
            <X className="w-4 h-4" />
          </Button>
        </div>
        {loadingConversations ? (
          <div className="flex items-center justify-center py-2">
            <Loader2 className="animate-spin text-black/50" />
          </div>
        ) : (
          <>
            {conversations.map((conv) => (
              editingConversationId !== conv.id ? (
                <div key={conv.id} className="flex flex-row items-center min-w-0 group hover:bg-white/20 px-3 rounded-lg">
                  <Button variant="ghost" className="p-0 text-sm font-semibold flex-1 min-w-0 justify-start hover:bg-transparent">
                    <span className="truncate">{conv.name || 'Untitled Conversation'}</span>
                  </Button>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="link" className="w-0 p-0 opacity-0 overflow-hidden group-hover:w-auto group-hover:px-2 group-hover:opacity-100 transition-none">
                        <Ellipsis className="w-4 h-4 text-black/50" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      <DropdownMenuGroup>
                        <DropdownMenuItem onClick={() => { setNewConversationName(conv.name || ''); setEditingConversationId(conv.id) }}><Pen className="mr-2" />Rename</DropdownMenuItem>
                        <DropdownMenuItem variant="destructive" onClick={() => { }}><Trash2 className="mr-2" />Delete</DropdownMenuItem>
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              ) : (
                <div key={conv.id} className="px-3">
                  <Input onSubmit={handleUpdateConversationName} onChange={(e) => setNewConversationName(e.target.value)} className="text-sm font-semibold" value={newConversationName} />
                </div>
              )
            ))}

            {/* Skeleton loaders for infinite scroll */}
            {hasMoreConversations && (
              <>
                {Array.from({ length: SKELETON_COUNT }).map((_, idx) => (
                  <div key={`skeleton-${idx}`} className="flex flex-row items-center min-w-0 px-3 py-2 rounded-lg">
                    <Skeleton className="h-5 w-full" />
                  </div>
                ))}
                <div ref={observerTarget} className="w-10 h-10 bg-black">HELO</div>
              </>
            )}
          </>
        )}
      </div>
    </ContentContainer>
  );
};