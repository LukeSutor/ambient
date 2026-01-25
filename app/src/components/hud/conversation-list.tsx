import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useWindows } from "@/lib/windows/useWindows";
import type { Conversation } from "@/types/conversations";
import { zodResolver } from "@hookform/resolvers/zod";
import { Ellipsis, Loader2, Pen, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { ContentContainer } from "./content-container";

const SKELETON_COUNT = 3;

const conversationNameSchema = z.object({
  name: z
    .string()
    .min(1, "Name cannot be empty")
    .max(100, "Name must be less than 100 characters"),
});

type ConversationNameFormValues = z.infer<typeof conversationNameSchema>;

interface ConversationListProps {
  conversations: Conversation[];
  hasMoreConversations: boolean;
  loadConversation: (id: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  loadMoreConversations: () => Promise<void>;
  renameConversation: (
    conversationId: string,
    newName: string,
  ) => Promise<void>;
}

export function ConversationList({
  conversations,
  hasMoreConversations,
  loadConversation,
  deleteConversation,
  loadMoreConversations,
  renameConversation,
}: ConversationListProps) {
  const [editingConversationId, setEditingConversationId] = useState<
    string | null
  >(null);
  const observerTarget = useRef<HTMLDivElement>(null);
  const isLoadingRef = useRef(false);

  const { setChatExpanded, toggleChatHistory } = useWindows();

  const form = useForm<ConversationNameFormValues>({
    resolver: zodResolver(conversationNameSchema),
    defaultValues: { name: "" },
  });

  // Escape key handler for editing
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setEditingConversationId(null);
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  // Infinite scroll observer
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (
          entries[0].isIntersecting &&
          hasMoreConversations &&
          !isLoadingRef.current
        ) {
          isLoadingRef.current = true;
          void loadMoreConversations().finally(() => {
            isLoadingRef.current = false;
          });
        }
      },
      { threshold: 0.1 },
    );

    const target = observerTarget.current;
    if (target) observer.observe(target);

    return () => {
      if (target) observer.unobserve(target);
    };
  }, [hasMoreConversations, loadMoreConversations]);

  const handleLoadConversation = useCallback(
    async (id: string) => {
      await loadConversation(id);
      setChatExpanded();
    },
    [loadConversation, setChatExpanded],
  );

  const handleUpdateConversationName = useCallback(
    async (values: ConversationNameFormValues) => {
      if (editingConversationId) {
        await renameConversation(editingConversationId, values.name);
      }
      setEditingConversationId(null);
    },
    [editingConversationId, renameConversation],
  );

  const startEditing = useCallback(
    (conv: Conversation) => {
      setEditingConversationId(conv.id);
      form.reset({ name: conv.name || "" });
    },
    [form],
  );

  const handleCloseChatHistory = useCallback(() => {
    void toggleChatHistory(false);
  }, [toggleChatHistory]);

  return (
    <ContentContainer>
      <div className="flex flex-col w-full h-full hud-scroll overflow-y-auto pt-2 px-2">
        <div className="flex flex-row justify-between items-center mb-2 ml-3 text-black/80">
          <p className="text-sm font-semibold whitespace-nowrap">
            Chat History
          </p>
          <Button
            onClick={handleCloseChatHistory}
            variant="ghost"
            size="icon"
            className="!p-2"
          >
            <X className="w-4 h-4" />
          </Button>
        </div>
        {conversations.length === 0 ? (
          isLoadingRef.current ? (
            <div className="flex items-center justify-center py-2">
              <Loader2 className="animate-spin text-black/50" />
            </div>
          ) : (
            <Empty>
              <EmptyHeader>
                <EmptyTitle>No Conversations</EmptyTitle>
                <EmptyDescription className="text-gray-600">
                  You have no conversations yet. Start a new chat to see your
                  conversation history here.
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          )
        ) : (
          <>
            {conversations.map((conv) =>
              editingConversationId !== conv.id ? (
                <div
                  key={conv.id}
                  className="flex flex-row items-center min-w-0 group hover:bg-white/20 px-3 rounded-lg"
                >
                  <Button
                    onClick={() => {
                      void handleLoadConversation(conv.id);
                    }}
                    variant="ghost"
                    className="p-0 text-sm font-semibold flex-1 min-w-0 justify-start hover:bg-transparent"
                  >
                    <span
                      title={conv.name || "Untitled Conversation"}
                      className="truncate"
                    >
                      {conv.name || "Untitled Conversation"}
                    </span>
                  </Button>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="link"
                        className="w-0 p-0 opacity-0 overflow-hidden group-hover:w-auto group-hover:px-2 group-hover:opacity-100 transition-none"
                      >
                        <Ellipsis className="w-4 h-4 text-black/50" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      <DropdownMenuGroup>
                        <DropdownMenuItem
                          onClick={() => {
                            startEditing(conv);
                          }}
                        >
                          <Pen className="mr-2" />
                          Rename
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          variant="destructive"
                          onClick={() => {
                            void deleteConversation(conv.id);
                          }}
                        >
                          <Trash2 className="mr-2" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuGroup>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              ) : (
                <div key={conv.id}>
                  <Form {...form}>
                    <form
                      onSubmit={(e) => {
                        void form.handleSubmit(handleUpdateConversationName)(e);
                      }}
                      className="space-y-2"
                    >
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
                                  if (
                                    Object.keys(form.formState.errors)
                                      .length === 0
                                  ) {
                                    void form.handleSubmit(
                                      handleUpdateConversationName,
                                    )();
                                  }
                                }}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") {
                                    e.preventDefault();
                                    void form.handleSubmit(
                                      handleUpdateConversationName,
                                    )();
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
              ),
            )}

            {/* Skeleton loaders for infinite scroll */}
            {hasMoreConversations && (
              <>
                {Array.from({ length: SKELETON_COUNT }).map((_, idx) => (
                  <div
                    // biome-ignore lint/suspicious/noArrayIndexKey: Skeletons are stable and don't reorder
                    key={`skeleton-${idx}`}
                    className="flex flex-row items-center min-w-0 px-3 py-2 rounded-lg"
                  >
                    <Skeleton className="h-5 w-full max-w-36" />
                  </div>
                ))}
                <div ref={observerTarget} className="w-1 h-1" />
              </>
            )}
          </>
        )}
      </div>
    </ContentContainer>
  );
}
