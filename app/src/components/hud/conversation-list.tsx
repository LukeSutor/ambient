import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Conversation } from '@/types/conversations';
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
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormMessage,
} from "@/components/ui/form";

const SKELETON_COUNT = 3;

const conversationNameSchema = z.object({
  name: z.string().min(1, "Name cannot be empty").max(100, "Name must be less than 100 characters"),
});

interface ConversationListProps {
  conversations: Conversation[];
  hasMoreConversations: boolean;
  loadConversation: (id: string) => Promise<void>;
  loadMoreConversations: () => Promise<void>;
  renameConversation: (conversationId: string, newName: string) => Promise<void>;
  toggleChatHistory: (nextState?: boolean) => Promise<void>;
}

export function ConversationList({ conversations, hasMoreConversations, loadConversation, loadMoreConversations, renameConversation, toggleChatHistory }: ConversationListProps) {
  // State
  const [loadingMore, setLoadingMore] = useState(false);
  const [editingConversationId, setEditingConversationId] = useState<string | null>(null);

  // Form setup
  const form = useForm<z.infer<typeof conversationNameSchema>>({
    resolver: zodResolver(conversationNameSchema),
    defaultValues: {
      name: "",
    },
  });

  // Refs
  const observerTarget = useRef<HTMLDivElement>(null);
  const isLoadingRef = useRef(false);

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

  // Intersection Observer for infinite scroll
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMoreConversations && !isLoadingRef.current) {
          console.log("loading more conversations");
          isLoadingRef.current = true;
          setLoadingMore(true);
          loadMoreConversations().finally(() => {
            isLoadingRef.current = false;
            setLoadingMore(false);
          });
        }
      },
      { threshold: 0.1 }
    );

    const currentTarget = observerTarget.current;
    if (currentTarget) {
      observer.observe(currentTarget);
    }

    return () => {
      if (currentTarget) {
        observer.unobserve(currentTarget);
      }
    };
  }, [observerTarget.current, hasMoreConversations, loadMoreConversations]);

  const handleUpdateConversationName = async (values: z.infer<typeof conversationNameSchema>) => {
    await renameConversation(editingConversationId!, values.name);
    setEditingConversationId(null);
  };

  const startEditing = (conv: Conversation) => {
    setEditingConversationId(conv.id);
    form.reset({ name: conv.name || '' });
  };

  return (
    <ContentContainer>
      <div
        className="flex flex-col w-full h-full hud-scroll overflow-y-auto pt-2 px-2"
      >
        <div className="flex flex-row justify-between items-center mb-2 ml-3 text-black/80">
          <p className="text-sm font-semibold whitespace-nowrap">Chat History</p>
          <Button onClick={() => toggleChatHistory(false)} variant="ghost" size="icon" className="!p-2">
            <X className="w-4 h-4" />
          </Button>
        </div>
        {conversations.length === 0 ? (
          <div className="flex items-center justify-center py-2">
            <Loader2 className="animate-spin text-black/50" />
          </div>
        ) : (
          <>
            {conversations.map((conv) => (
              editingConversationId !== conv.id ? (
                <div key={conv.id} className="flex flex-row items-center min-w-0 group hover:bg-white/20 px-3 rounded-lg">
                  <Button onClick={() => loadConversation(conv.id)} variant="ghost" className="p-0 text-sm font-semibold flex-1 min-w-0 justify-start hover:bg-transparent">
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
                        <DropdownMenuItem onClick={() => startEditing(conv)}><Pen className="mr-2" />Rename</DropdownMenuItem>
                        <DropdownMenuItem variant="destructive" onClick={() => { }}><Trash2 className="mr-2" />Delete</DropdownMenuItem>
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              ) : (
                <div key={conv.id}>
                  <Form {...form}>
                    <form onSubmit={form.handleSubmit(handleUpdateConversationName)} className="space-y-2">
                      <FormField
                        control={form.control}
                        name="name"
                        render={({ field }) => (
                          <FormItem>
                            <FormControl>
                              <Input 
                                {...field} 
                                className="text-sm font-semibold h-8" 
                                autoFocus
                                onBlur={() => {
                                  // Submit on blur if there are no errors
                                  if (Object.keys(form.formState.errors).length === 0) {
                                    form.handleSubmit(handleUpdateConversationName)();
                                  }
                                }}
                                onKeyDown={(e) => {
                                  if (e.key === 'Enter') {
                                    e.preventDefault();
                                    form.handleSubmit(handleUpdateConversationName)();
                                  }
                                }}
                              />
                            </FormControl>
                            <FormMessage className="text-xs" />
                          </FormItem>
                        )}
                      />
                    </form>
                  </Form>
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
                <div ref={observerTarget} className="w-1 h-1"></div>
              </>
            )}
          </>
        )}
      </div>
    </ContentContainer>
  );
};