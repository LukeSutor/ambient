"use client";

import type { Conversation, Message, Role } from "@/types/conversations";
import type { AttachmentData } from "@/types/events";
import type { MemoryEntry } from "@/types/memory";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";
import { useConversationContext } from "./ConversationProvider";
import {
  createConversation,
  deleteConversation as deleteApiConversation,
  emitGenerateConversationName,
  ensureLlamaServerRunning,
  sendMessage as sendChatApiMessage,
  sendAgentMessage,
  startComputerUseSession,
  stopComputerUseSession,
} from "./api";
import type { ChatMessage } from "./types";

const CONVERSATION_LIMIT = 20;
const OCR_TIMEOUT_MS = 10000;

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
  };
}

/**
 * Creates a new user message
 */
function createUserMessage(
  content: string,
  conversationId: string,
): ChatMessage {
  const message: Message = {
    id: crypto.randomUUID(),
    conversation_id: conversationId,
    role: "user" as Role,
    content,
    timestamp: new Date().toISOString(),
    attachments: [],
    memory: null,
    message_type: "text",
    metadata: null,
  };
  return {
    message,
    reasoningMessages: [],
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
  const isLoadingMoreRef = useRef(false);

  // Auto-scroll effect for streaming messages
  useEffect(() => {
    if (state.isStreaming && messagesEndRef?.current) {
      // Accessing streamingContent ensures this effect re-runs on every update
      void state.streamingContent;
      queueMicrotask(() => {
        messagesEndRef.current?.scrollIntoView({
          behavior: "smooth",
          block: "end",
        });
      });
    }
  }, [state.streamingContent, state.isStreaming, messagesEndRef]);

  // Initialization effect

  useEffect(() => {
    // Check shared initialization ref to prevent multiple initializations
    if (state.initializationRef.current) {
      return;
    }
    state.initializationRef.current = true;

    const initialize = async () => {
      console.log("[useConversation] Initializing...");

      // Load the conversations list immediately
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

      // Ensure llama server is running in the background - don't block the UI
      void ensureLlamaServerRunning();
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
            if (msg.message.role === "tool") {
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
        } else if (state.conversationType === "agent") {
          await sendAgentMessage(
            activeConversationId,
            content,
            attachmentData,
            userMessage.message.id,
          );
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
   * Toggle Agentic Mode
   */
  const toggleAgenticMode = useCallback((): void => {
    if (state.conversationType === "chat") {
      dispatch({ type: "SET_CONVERSATION_TYPE", payload: "agent" });
    } else {
      dispatch({ type: "SET_CONVERSATION_TYPE", payload: "chat" });
    }
  }, [dispatch, state.conversationType]);

  /**
   * Sets the conversation type
   */
  const setConversationType = useCallback(
    (type: string): void => {
      dispatch({ type: "SET_CONVERSATION_TYPE", payload: type });
    },
    [dispatch],
  );

  /**
   * Stops the current computer use session
   */
  const stopComputerUse = useCallback(async (): Promise<void> => {
    try {
      await stopComputerUseSession();
    } catch (error) {
      console.error("[useConversation] Failed to stop computer use:", error);
    }
  }, []);

  /**
   * Add attachment data
   */
  const addAttachmentData = useCallback(
    (attachment: AttachmentData): void => {
      // Check if attachment with same name already exists
      const exists = state.attachmentData.some(
        (att) => att.name === attachment.name,
      );
      if (exists) {
        toast.error(`Attachment "${attachment.name}" already added.`);
        return;
      }
      dispatch({ type: "ADD_ATTACHMENT_DATA", payload: attachment });
    },
    [dispatch, state.attachmentData],
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
    refreshConversations,
    renameConversation,
    dispatchOCRCapture,
    toggleComputerUse,
    toggleAgenticMode,
    setConversationType,
    stopComputerUse,
    addAttachmentData,
    removeAttachmentData,
  };
}
