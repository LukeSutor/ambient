'use client';

import React, { useState, useEffect, useRef, forwardRef, useCallback } from 'react';
import Markdown from 'react-markdown';
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import AnimatedText from '@/components/ui/animated-text';
import { HoverCard, HoverCardTrigger, HoverCardContent } from '@/components/ui/hover-card';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { NotebookPen, Loader2, Ellipsis, Trash2, Pen } from 'lucide-react';
import { useConversation } from '@/lib/conversations';
import { useWindows } from '@/lib/windows/useWindows';
import { Separator } from '../ui/separator';
import { Conversation } from '@/types/conversations';
import { HudDimensions } from '@/types/settings';

interface MessageListProps {
  hudDimensions: HudDimensions | null;
  showMarkdown?: boolean; // Allow turning off markdown for perf if desired
}

const CONVERSATION_LIMIT = 20;
const SKELETON_COUNT = 3;

// Container element forwards ref to the tail sentinel to support scrollIntoView
export const MessageList = forwardRef<HTMLDivElement, MessageListProps>(
  ({ hudDimensions, showMarkdown = true }, endRef) => {
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
    setExpandedChat,
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

  // Intersection Observer for infinite scroll
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMoreConversations && !loadingMore) {
          loadMoreConversations();
        }
      },
      { threshold: 0.1 }
    );

    const currentTarget = observerTarget.current;
    if (currentTarget) {
      console.log("observing")
      observer.observe(currentTarget);
    } else {
      console.log("not observing")
    }

    return () => {
      if (currentTarget) {
        observer.unobserve(currentTarget);
      }
    };
  }, [hasMoreConversations, loadingMore, loadMoreConversations, isChatHistoryExpanded]);

  const handleUpdateConversationName = async () => {
    console.log("updating");
  };

  // Helper function to check if previous message has memory
  const hasPreviousMemory = (index: number) => {
    return index > 0 && messages[index - 1]?.role === 'user' && messages[index - 1]?.memory !== null;
  };

  const maxHeight = hudDimensions ? `${hudDimensions.chat_max_height}px` : '500px';
  const minHeight = isChatHistoryExpanded && conversations.length === 0 && !loadingConversations ? '64px' : undefined;

  return (
    <div className="flex flex-row w-full h-full gap-0">
      {/* Chat history - Left scroll container */}
      <div className={`flex flex-row transition-all duration-300 overflow-hidden ${isChatHistoryExpanded ? (messages.length > 0 && isChatExpanded ? "w-1/2" : "w-full") : "w-0 h-0"}`}>
        <div 
          className="flex flex-col w-full hud-scroll overflow-y-auto pr-2"
          style={{ maxHeight, minHeight }}
        >
          <p className="text-sm text-black/50 font-semibold whitespace-nowrap mb-2 ml-3">Chat History</p>
          {loadingConversations ? (
            <div className="flex items-center justify-center py-4">
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
                          <DropdownMenuItem onClick={() => {setNewConversationName(conv.name || ''); setEditingConversationId(conv.id)}}><Pen className="mr-2" />Rename</DropdownMenuItem>
                          <DropdownMenuItem variant="destructive" onClick={() => {}}><Trash2 className="mr-2" />Delete</DropdownMenuItem>
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
        {messages.length > 0 &&
          <Separator orientation="vertical" decorative={true} className="bg-black/20 mx-2 h-full" />
        }
      </div>

      {/* Message list */}
      <div 
        className={`flex flex-col hud-scroll overflow-y-auto transition-all duration-300 ${messages.length > 0 ? "w-full" : "w-0 overflow-hidden"}`}
        style={{ maxHeight }}
      >
        <div className="flex flex-col space-y-2 px-2">
          {messages.map((m, i) => (
            <div
              key={`m-${i}`}
              className={
                m.role === 'user'
                  ? 'max-w-[85%] ml-auto grid transition-[grid-template-rows] duration-300 ease-out'
                  : 'max-w-[95%] w-full text-left mx-auto grid transition-[grid-template-rows] duration-300 ease-out'
              }
              style={{
                gridTemplateRows: m.content ? '1fr' : '0fr'
              }}
            >
              {m.role === 'user' ? (
                <div className="overflow-hidden bg-white/60 border border-black/20 rounded-xl px-3 py-2">
                  <div className="whitespace-pre-wrap">{m.content}</div>
                </div>
              ) : (
                <div className="overflow-hidden">
                  {/* Always reserve space for the memory indicator area */}
                  <div className="h-4 flex items-center justify-start -mb-2">
                    {hasPreviousMemory(i) ? (
                      <HoverCard>
                        <HoverCardTrigger asChild>
                          <div className="flex items-center gap-1 text-xs text-black/50">
                            <NotebookPen className="h-4 w-4" />
                            <span className='font-bold'>Updated saved memory</span>
                          </div>
                        </HoverCardTrigger>
                        <HoverCardContent side="top" className="w-min max-w-80 bg-white/70">
                          <div className="space-y-3">
                            <div>
                              <p className="text-sm text-black">
                                {messages[i - 1]?.memory?.text || 'No memory text available'}
                              </p>
                            </div>
                            <Button 
                              variant="outline" 
                              size="sm" 
                              className="w-full bg-white/50"
                              onClick={(e) => {
                                e.preventDefault();
                                // TODO: Implement manage memories functionality
                              }}
                            >
                              Manage Memories
                            </Button>
                          </div>
                        </HoverCardContent>
                      </HoverCard>
                    ) : (
                      <div className="h-4 w-4" />
                    )}
                  </div>
                  {showMarkdown ? (
                    <Markdown {...llmMarkdownConfig}>{m.content}</Markdown>
                  ) : (
                    <div className="prose prose-sm max-w-none">
                      <AnimatedText content={m.content} />
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
          <div ref={endRef} />
        </div>
      </div>
    </div>
  );
}
);

MessageList.displayName = 'MessageList';

export default MessageList;
