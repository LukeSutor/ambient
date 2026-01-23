"use client";

import type { Conversation, Message, Role } from "@/types/conversations";
import type {
  AttachmentData,
  AttachmentsCreatedEvent,
  ChatStreamEvent,
  ComputerUseUpdateEvent,
  MemoryExtractedEvent,
  OcrResponseEvent,
  RenameConversationEvent,
} from "@/types/events";
import type { MemoryEntry } from "@/types/memory";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef } from "react";
import { useConversationContext } from "./ConversationProvider";
import {
  createConversation,
  deleteConversation as deleteApiConversation,
  emitGenerateConversationName,
  ensureLlamaServerRunning,
  sendMessage as sendChatApiMessage,
  startComputerUseSession,
} from "./api";
import type { ChatMessage } from "./types";

const CONVERSATION_LIMIT = 20;
const OCR_TIMEOUT_MS = 10000;

/**
 * Extracts and removes <think> tags from LLM responses
 */
function extractThinkingContent(text: string): string {
  // Return early if no <think> tags
  if (!text.includes("<think>")) {
    return text;
  }
  const thinkStartIndex = text.indexOf("<think>");
  const thinkEndIndex = text.indexOf("</think>");

  let cleanText = text;

  if (thinkStartIndex !== -1) {
    if (thinkEndIndex !== -1) {
      // Remove complete thinking block
      cleanText =
        text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
    } else {
      // Remove incomplete thinking block
      cleanText = text.substring(0, thinkStartIndex);
    }
  }

  return cleanText;
}

/**
 * Transforms a backend message format to frontend ChatMessage format
 */
function transformBackendMessage(backendMessage: Message): ChatMessage {
  return {
    message: {
      ...backendMessage,
      role: backendMessage.role.toLowerCase() as Role,
    },
    reasoningMessages: [],
    memory: null,
  };
}

/**
 * Creates a new user message
 */
function createUserMessage(
  content: string,
  conversationId: string,
  memory: MemoryEntry | null = null,
): ChatMessage {
  const message: Message = {
    id: crypto.randomUUID(),
    conversation_id: conversationId,
    role: "user" as Role,
    content,
    timestamp: new Date().toISOString(),
    attachments: [],
  };
  return {
    message,
    reasoningMessages: [],
    memory,
  };
}

/**
 * Main conversation hook - provides all conversation functionality
 *
 * @param messagesEndRef - Optional ref for auto-scrolling to bottom on new messages
 * @returns Conversation state and operations
 */
export function useConversation(
  messagesEndRef?: React.RefObject<HTMLDivElement | null>,
) {
  const { state, dispatch } = useConversationContext();
  const cleanupRef = useRef<(() => void) | null>(null);
  const isLoadingMoreRef = useRef(false);

  // Use refs for values needed in listeners to avoid re-registering
  const convIdRef = useRef(state.conversationId);
  useEffect(() => {
    convIdRef.current = state.conversationId;
  }, [state.conversationId]);

  // ============================================================
  // Event Listeners Setup
  // ============================================================

  useEffect(() => {
    let isMounted = true;
    const unlisteners: UnlistenFn[] = [];

    const setupEvents = async () => {
      // Clean up previous listeners
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }

      if (!isMounted) return;

      try {
        console.log("[useConversation] Setting up event listeners...");

        // Register all listeners in parallel to avoid race conditions
        const listenerPromises = [
          // Stream Listener
          listen<ChatStreamEvent>("chat_stream", (event) => {
            const { delta, full_response, is_finished, conv_id } =
              event.payload;

            // Filter by conversation ID using ref to prevent corruption
            if (conv_id && conv_id !== convIdRef.current) {
              return;
            }

            if (is_finished) {
              // Stream is complete
              const finalText = extractThinkingContent(full_response);
              dispatch({ type: "FINALIZE_STREAM", payload: finalText });
              return;
            }

            if (delta) {
              // Update content
              const cleanContent = extractThinkingContent(full_response);
              dispatch({
                type: "UPDATE_STREAMING_CONTENT",
                payload: cleanContent,
              });

              // Auto-scroll to bottom
              if (messagesEndRef?.current) {
                queueMicrotask(() => {
                  messagesEndRef.current?.scrollIntoView({
                    behavior: "smooth",
                    block: "end",
                  });
                });
              }
            }
          }),

          // Computer Use Listener
          listen<ComputerUseUpdateEvent>("computer_use_update", (event) => {
            // Create message from event
            const chatMessage: ChatMessage = {
              message: event.payload.message,
              reasoningMessages: [],
              memory: null,
            };

            if (event.payload.status === "completed") {
              dispatch({
                type: "FINALIZE_STREAM",
                payload: event.payload.message.content,
              });
            } else {
              dispatch({ type: "ADD_REASONING_MESSAGE", payload: chatMessage });
            }
          }),

          // Memory Listener
          listen<MemoryExtractedEvent>("memory_extracted", (event) => {
            const { memory } = event.payload;

            // Attach memory to the message with matching message_id
            if (memory.message_id) {
              dispatch({
                type: "ATTACH_MEMORY",
                payload: { messageId: memory.message_id, memory },
              });
            }
          }),

          // OCR Listener
          listen<OcrResponseEvent>("ocr_response", (event) => {
            //TODO: use the ocr loading state to show a skeleton ocr response in the input box
            if (event.payload.success && event.payload.text) {
              const ocrData: AttachmentData = {
                name: "Screen Capture",
                file_type: "ambient/ocr",
                data: event.payload.text,
              };
              dispatch({ type: "ADD_ATTACHMENT_DATA", payload: ocrData });
            }

            // Stop OCR loading state and clear timeout
            dispatch({ type: "SET_OCR_LOADING", payload: false });
            dispatch({ type: "CLEAR_OCR_TIMEOUT" });
          }),

          // Attachments created listener
          listen<AttachmentsCreatedEvent>("attachments_created", (event) => {
            const { attachments } = event.payload;
            dispatch({
              type: "ADD_ATTACHMENTS_TO_MESSAGE",
              payload: { messageId: event.payload.message_id, attachments },
            });
          }),

          // Rename conversation listener
          listen<RenameConversationEvent>("rename_conversation", (event) => {
            const { conv_id, new_name } = event.payload;
            dispatch({
              type: "RENAME_CONVERSATION",
              payload: { id: conv_id, newName: new_name },
            });
          }),
        ];

        const results = await Promise.all(listenerPromises);
        unlisteners.push(...results);

        console.log("[useConversation] Event listeners initialized");
      } catch (error) {
        console.error("[useConversation] Failed to setup events:", error);
      }
    };

    void setupEvents();

    // Store cleanup function
    cleanupRef.current = () => {
      // Clear any existing OCR timeout
      dispatch({ type: "CLEAR_OCR_TIMEOUT" });

      for (const unlisten of unlisteners) {
        try {
          unlisten();
        } catch (error) {
          console.error("[useConversation] Error during cleanup:", error);
        }
      }
      console.log("[useConversation] Event listeners cleaned up");
    };

    // Cleanup on unmount
    return () => {
      isMounted = false;

      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
    };
  }, [dispatch, messagesEndRef]); // Removed state.conversationId from dependencies

  // ============================================================
  // Initialization Effect
  // ============================================================

  useEffect(() => {
    // Check shared initialization ref to prevent multiple initializations
    if (state.initializationRef.current) {
      return;
    }
    state.initializationRef.current = true;

    const initialize = async () => {
      console.log("[useConversation] Initializing...");

      // Ensure llama server is running
      await ensureLlamaServerRunning();

      // Load the conversations list
      try {
        const conversations = await invoke<Conversation[]>(
          "list_conversations",
          {
            limit: CONVERSATION_LIMIT,
            offset: 0,
          },
        );
        if (conversations.length < CONVERSATION_LIMIT) {
          dispatch({ type: "SET_NO_MORE_CONVERSATIONS" });
        }
        dispatch({ type: "SET_CONVERSATIONS", payload: conversations });
      } catch (error) {
        console.error("[useConversation] Failed to load conversations:", error);
      }
    };

    void initialize();
  }, [dispatch, state.initializationRef]); // Depend on the shared ref

  // ============================================================
  // Operations
  // ============================================================

  /**
   * Resets the conversation state
   */
  const resetConversation = useCallback(
    async (delay?: number): Promise<string | null> => {
      try {
        dispatch({ type: "SET_CONVERSATION_ID", payload: null });
        if (delay && delay > 0) {
          setTimeout(() => {
            dispatch({ type: "CLEAR_MESSAGES" });
          }, delay);
        } else {
          dispatch({ type: "CLEAR_MESSAGES" });
        }
        return null;
      } catch (error) {
        console.error(
          "[useConversation] Failed to create conversation:",
          error,
        );
        return null;
      }
    },
    [dispatch],
  );

  /**
   * Deletes a conversation by ID
   */
  const deleteConversation = useCallback(
    async (id: string): Promise<void> => {
      try {
        await deleteApiConversation(id);
        dispatch({ type: "DELETE_CONVERSATION", payload: { id } });
      } catch (error) {
        console.error(
          "[useConversation] Failed to delete conversation:",
          error,
        );
      }
    },
    [dispatch],
  );

  /**
   * Loads a conversation by ID
   */
  const loadConversation = useCallback(
    async (id: string): Promise<void> => {
      try {
        const conversation = await invoke<Conversation>("get_conversation", {
          conversationId: id,
        });
        dispatch({ type: "LOAD_CONVERSATION", payload: conversation });
        await loadMessages(conversation);
      } catch (error) {
        console.error("[useConversation] Failed to load conversation:", error);
      }
    },
    [dispatch],
  );

  /**
   * Loads messages for a specific conversation
   */
  const loadMessages = useCallback(
    async (conversation: Conversation): Promise<void> => {
      try {
        const backendMessages = await invoke<Message[]>("get_messages", {
          conversationId: conversation.id,
        });
        let messages = backendMessages.map(transformBackendMessage);
        // Load messages depending on conversation type
        if (conversation.conv_type === "computer_use") {
          // Collect all assistant/function messages into the last assistant message's reasoningMessages
          const finalizedMessages: ChatMessage[] = [];
          let currentAssistantMessage: ChatMessage | null = null;
          // Loop reverse through the messages to group reasoning by each final assistant message
          const reversedMessages = [...messages].reverse();
          for (const msg of reversedMessages) {
            if (msg.message.role === "functioncall") {
              // Add function reasoning if current assistant message
              if (currentAssistantMessage) {
                currentAssistantMessage.reasoningMessages.unshift(msg);
              }
            } else if (msg.message.role === "user") {
              // Add current assistant message to finalized and reset
              if (currentAssistantMessage) {
                finalizedMessages.unshift(currentAssistantMessage);
                finalizedMessages.unshift(msg);
                currentAssistantMessage = null;
              }
            } else if (msg.message.role === "assistant") {
              // If there is a current assistant message, add this as reasoning
              if (currentAssistantMessage) {
                currentAssistantMessage.reasoningMessages.unshift(msg);
              } else {
                currentAssistantMessage = msg;
              }
            }
          }
          // If there is a remaining assistant message, add it
          if (currentAssistantMessage) {
            finalizedMessages.unshift(currentAssistantMessage);
          }
          messages = finalizedMessages;
        }
        dispatch({ type: "LOAD_MESSAGES", payload: messages });
      } catch (error) {
        console.error("[useConversation] Failed to load messages:", error);
      }
    },
    [dispatch],
  );

  /**
   * Sends a message
   */
  const sendMessage = useCallback(
    async (conversationId: string | null, content: string): Promise<void> => {
      try {
        // Validate message
        if (!content.trim()) {
          console.warn("[useConversation] Empty message, skipping send");
          return;
        }

        // Create conversation and generate name if first message
        let activeConversationId = conversationId;
        if (!activeConversationId) {
          const conversation = await createConversation(
            undefined,
            state.conversationType,
          );
          activeConversationId = conversation.id;
          dispatch({ type: "SET_CONVERSATION_ID", payload: conversation.id });
          dispatch({ type: "PREPEND_CONVERSATION", payload: conversation });
          await emitGenerateConversationName(conversation.id, content);
        }

        // Create user message with ID and timestamp
        const userMessage = createUserMessage(content, activeConversationId);

        // Clear attachment data
        const attachmentData = state.attachmentData;
        dispatch({ type: "CLEAR_ATTACHMENT_DATA" });

        // Start user message with empty content (for animation)
        dispatch({
          type: "START_USER_MESSAGE",
          payload: {
            id: userMessage.message.id,
            conversationId: activeConversationId,
            timestamp: userMessage.message.timestamp,
          },
        });

        // Use requestAnimationFrame to ensure the empty state is rendered first
        requestAnimationFrame(() => {
          // Then fill in the content to trigger the grid animation
          dispatch({
            type: "FINALIZE_USER_MESSAGE",
            payload: {
              id: userMessage.message.id,
              content: userMessage.message.content,
            },
          });
        });

        dispatch({
          type: "START_ASSISTANT_MESSAGE",
          payload: { conversationId: activeConversationId },
        });
        dispatch({ type: "SET_LOADING", payload: true });
        dispatch({ type: "SET_STREAMING", payload: true });

        // Send hud chat or computer use event
        if (state.conversationType === "computer_use") {
          void startComputerUseSession(activeConversationId, content);
        } else {
          await sendChatApiMessage(
            activeConversationId,
            content,
            attachmentData,
            userMessage.message.id,
          );
        }
      } catch (error) {
        console.error("[useConversation] Failed to send message:", error);

        // Remove the placeholder assistant message on error
        dispatch({
          type: "FINALIZE_STREAM",
          payload: "[Error generating response]",
        });
        dispatch({ type: "SET_LOADING", payload: false });
        dispatch({ type: "SET_STREAMING", payload: false });
      }
    },
    [dispatch, state.conversationType, state.attachmentData],
  );

  /**
   * Get all conversations
   */
  const loadMoreConversations = useCallback(async (): Promise<void> => {
    // Prevent concurrent calls
    if (isLoadingMoreRef.current || !state.hasMoreConversations) {
      return;
    }

    isLoadingMoreRef.current = true;

    try {
      const nextPage = state.conversationPage + 1;
      const offset = nextPage * CONVERSATION_LIMIT;
      const conversations = await invoke<Conversation[]>("list_conversations", {
        limit: CONVERSATION_LIMIT,
        offset,
      });
      if (conversations.length < CONVERSATION_LIMIT) {
        // No more conversations to load
        dispatch({ type: "SET_NO_MORE_CONVERSATIONS" });
      }
      dispatch({ type: "ADD_CONVERSATIONS", payload: conversations });
      dispatch({ type: "INCREMENT_CONVERSATION_PAGE" });
    } catch (error) {
      console.error(
        "[useConversation] Failed to load more conversations:",
        error,
      );
    } finally {
      isLoadingMoreRef.current = false;
    }
  }, [dispatch, state.conversationPage, state.hasMoreConversations]);

  /**
   * Refresh conversations list based on current page
   */
  const refreshConversations = useCallback(async (): Promise<void> => {
    try {
      const conversations = await invoke<Conversation[]>("list_conversations", {
        limit: CONVERSATION_LIMIT * (state.conversationPage + 1),
        offset: 0,
      });
      if (
        conversations.length <
        CONVERSATION_LIMIT * (state.conversationPage + 1)
      ) {
        dispatch({ type: "SET_NO_MORE_CONVERSATIONS" });
      }
      dispatch({ type: "SET_CONVERSATIONS", payload: conversations });
    } catch (error) {
      console.error(
        "[useConversation] Failed to refresh conversations:",
        error,
      );
    }
  }, [dispatch, state.conversationPage]);

  /**
   * Rename a conversation
   */
  const renameConversation = useCallback(
    async (id: string, newName: string): Promise<void> => {
      try {
        await invoke("update_conversation_name", {
          conversationId: id,
          name: newName,
        });
        dispatch({ type: "RENAME_CONVERSATION", payload: { id, newName } });
        await refreshConversations();
      } catch (error) {
        console.error(
          "[useConversation] Failed to rename conversation:",
          error,
        );
      }
    },
    [dispatch, refreshConversations],
  );

  /**
   * Dispatch an OCR capture event
   */
  const dispatchOCRCapture = useCallback(async (): Promise<void> => {
    dispatch({ type: "CLEAR_OCR_TIMEOUT" });
    dispatch({ type: "SET_OCR_LOADING", payload: true });
    try {
      await invoke("open_screen_selector");
      // Start a 10s timeout; if no OCR result arrives, stop loading
      const ocrTimeout = setTimeout(() => {
        console.warn("OCR capture timed out after 10s.");
        dispatch({ type: "SET_OCR_LOADING", payload: false });
        dispatch({ type: "CLEAR_OCR_TIMEOUT" });
      }, OCR_TIMEOUT_MS);
      dispatch({ type: "SET_OCR_TIMEOUT", payload: ocrTimeout });
    } catch (error: unknown) {
      console.error("Failed to open screen selector:", error);
      dispatch({ type: "SET_OCR_LOADING", payload: false });
      dispatch({ type: "CLEAR_OCR_TIMEOUT" });
    }
  }, [dispatch]);

  /**
   * Toggle Computer Use mode
   */
  const toggleComputerUse = useCallback((): void => {
    if (state.conversationType === "chat") {
      dispatch({ type: "SET_CONVERSATION_TYPE", payload: "computer_use" });
    } else {
      dispatch({ type: "SET_CONVERSATION_TYPE", payload: "chat" });
    }
  }, [dispatch, state.conversationType]);

  /**
   * Add attachment data
   */
  const addAttachmentData = useCallback(
    (attachment: AttachmentData): void => {
      dispatch({ type: "ADD_ATTACHMENT_DATA", payload: attachment });
    },
    [dispatch],
  );

  /**
   * Remove attachment data by index
   */
  const removeAttachmentData = useCallback(
    (index: number): void => {
      dispatch({ type: "REMOVE_ATTACHMENT_DATA", payload: index });
    },
    [dispatch],
  );

  // ============================================================
  // Return API
  // ============================================================

  return {
    // State
    ...state,

    // Operations
    resetConversation,
    deleteConversation,
    loadConversation,
    loadMessages,
    sendMessage,
    loadMoreConversations,
    renameConversation,
    dispatchOCRCapture,
    toggleComputerUse,
    addAttachmentData,
    removeAttachmentData,
  };
}
