"use client";

import type { Attachment, Conversation } from "@/types/conversations";
import type {
  AttachmentData,
  AttachmentsCreatedEvent,
  ChatStreamEvent,
  ComputerUseUpdateEvent,
  MemoryExtractedEvent,
  OcrResponseEvent,
  RenameConversationEvent,
  SkillActivatedEvent,
  ToolExecutionStartedEvent,
  ToolExecutionCompletedEvent,
} from "@/types/events";
import type { MemoryEntry } from "@/types/memory";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import type React from "react";
import {
  type ReactNode,
  createContext,
  useContext,
  useEffect,
  useReducer,
  useRef,
} from "react";
import type { ChatMessage, ConversationState } from "./types";

/**
 * Initial state for conversations
 */
/**
 * Extracts and removes <think> tags from LLM responses
 */
function extractThinkingContent(text: string): string {
  if (!text.includes("<think>")) {
    return text;
  }
  const thinkStartIndex = text.indexOf("<think>");
  const thinkEndIndex = text.indexOf("</think>");

  let cleanText = text;

  if (thinkStartIndex !== -1) {
    if (thinkEndIndex !== -1) {
      cleanText =
        text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
    } else {
      cleanText = text.substring(0, thinkStartIndex);
    }
  }

  return cleanText;
}

const initialState: ConversationState = {
  conversationId: null,
  conversationName: "",
  conversationType: "chat",
  messages: [],
  attachmentData: [],
  isStreaming: false,
  isLoading: false,
  streamingContent: "",
  ocrLoading: false,
  ocrTimeoutRef: { current: null },
  conversations: [],
  conversationPage: 0,
  hasMoreConversations: true,
  initializationRef: { current: false },
};

/**
 * Action types for the conversation reducer
 */
type ConversationAction =
  | { type: "SET_CONVERSATION_ID"; payload: string | null }
  | { type: "SET_CONVERSATION_TYPE"; payload: string }
  | { type: "SET_CONVERSATIONS"; payload: Conversation[] }
  | { type: "ADD_CONVERSATIONS"; payload: Conversation[] }
  | { type: "PREPEND_CONVERSATION"; payload: Conversation }
  | { type: "INCREMENT_CONVERSATION_PAGE" }
  | { type: "RENAME_CONVERSATION"; payload: { id: string; newName: string } }
  | { type: "DELETE_CONVERSATION"; payload: { id: string } }
  | { type: "SET_NO_MORE_CONVERSATIONS" }
  | { type: "LOAD_CONVERSATION"; payload: Conversation }
  | { type: "LOAD_MESSAGES"; payload: ChatMessage[] }
  | { type: "ADD_CHAT_MESSAGE"; payload: ChatMessage }
  | { type: "ADD_AGENTIC_MESSAGE"; payload: ChatMessage }
  | {
      type: "START_USER_MESSAGE";
      payload: { id: string; conversationId: string; timestamp: string };
    }
  | { type: "FINALIZE_USER_MESSAGE"; payload: { id: string; content: string } }
  | { type: "START_ASSISTANT_MESSAGE"; payload: { conversationId: string } }
  | { type: "UPDATE_STREAMING_CONTENT"; payload: string }
  | { type: "FINALIZE_STREAM"; payload: string }
  | { type: "ADD_ATTACHMENT_DATA"; payload: AttachmentData }
  | { type: "REMOVE_ATTACHMENT_DATA"; payload: number }
  | { type: "CLEAR_ATTACHMENT_DATA" }
  | {
      type: "ADD_ATTACHMENTS_TO_MESSAGE";
      payload: { messageId: string; attachments: Attachment[] };
    }
  | {
      type: "ATTACH_MEMORY";
      payload: { messageId: string; memory: MemoryEntry };
    }
  | { type: "SET_OCR_TIMEOUT"; payload: ReturnType<typeof setTimeout> | null }
  | { type: "CLEAR_OCR_TIMEOUT" }
  | { type: "CLEAR_MESSAGES" }
  | { type: "SET_LOADING"; payload: boolean }
  | { type: "SET_STREAMING"; payload: boolean }
  | { type: "SET_OCR_LOADING"; payload: boolean };

/**
 * Conversation reducer - handles all state updates
 */
function conversationReducer(
  state: ConversationState,
  action: ConversationAction,
): ConversationState {
  switch (action.type) {
    case "SET_CONVERSATION_ID":
      return {
        ...state,
        conversationId: action.payload,
      };

    case "SET_CONVERSATION_TYPE":
      return {
        ...state,
        conversationType: action.payload,
      };

    case "RENAME_CONVERSATION":
      return {
        ...state,
        conversationName: action.payload.newName,
        conversations: state.conversations.map((conv) =>
          conv.id === action.payload.id
            ? { ...conv, name: action.payload.newName }
            : conv,
        ),
      };

    case "DELETE_CONVERSATION":
      return {
        ...state,
        conversations: state.conversations.filter(
          (conv) => conv.id !== action.payload.id,
        ),
      };

    case "SET_CONVERSATIONS":
      return {
        ...state,
        conversations: action.payload,
      };

    case "ADD_CONVERSATIONS": {
      return {
        ...state,
        conversations: [...state.conversations, ...action.payload],
      };
    }

    case "PREPEND_CONVERSATION":
      return {
        ...state,
        conversations: [action.payload, ...state.conversations],
      };

    case "INCREMENT_CONVERSATION_PAGE":
      return {
        ...state,
        conversationPage: state.conversationPage + 1,
      };

    case "SET_NO_MORE_CONVERSATIONS":
      return {
        ...state,
        hasMoreConversations: false,
      };

    case "LOAD_CONVERSATION":
      return {
        ...state,
        conversationName: action.payload.name,
        conversationId: action.payload.id,
        conversationType: action.payload.conv_type,
      };

    case "LOAD_MESSAGES":
      return {
        ...state,
        messages: action.payload,
      };

    case "ADD_CHAT_MESSAGE":
      return {
        ...state,
        messages: [...state.messages, action.payload],
      };

    case "ADD_AGENTIC_MESSAGE": {
      const messages = [...state.messages];
      const lastIdx = messages.length - 1;

      // If the last message is an assistant text message, insert BEFORE it
      // this ensures thinking/tools appear above the final response
      if (lastIdx >= 0) {
        const lastMsg = messages[lastIdx];
        const role = lastMsg.message.role.toLowerCase();
        const mType = (lastMsg.message.message_type || "").toLowerCase();

        if (role === "assistant" && mType === "text") {
          messages.splice(lastIdx, 0, action.payload);
          return { ...state, messages };
        }
      }

      return {
        ...state,
        messages: [...state.messages, action.payload],
      };
    }

    case "START_USER_MESSAGE": {
      const newUserMessage: ChatMessage = {
        message: {
          id: action.payload.id,
          conversation_id: action.payload.conversationId,
          role: "user",
          content: "",
          timestamp: action.payload.timestamp,
          attachments: [],
          memory: null,
          message_type: "text",
          metadata: null,
        },
      };
      return {
        ...state,
        messages: [...state.messages, newUserMessage],
      };
    }

    case "FINALIZE_USER_MESSAGE": {
      // Find user message by ID and update its content
      const updatedMessages = state.messages.map((msg) => {
        if (
          msg.message.id === action.payload.id &&
          msg.message.role === "user"
        ) {
          return {
            ...msg,
            message: {
              ...msg.message,
              content: action.payload.content,
            },
          };
        }
        return msg;
      });

      return {
        ...state,
        messages: updatedMessages,
      };
    }

    case "START_ASSISTANT_MESSAGE": {
      const newMessage: ChatMessage = {
        message: {
          id: crypto.randomUUID(),
          conversation_id: action.payload.conversationId,
          role: "assistant",
          content: "",
          timestamp: new Date().toISOString(),
          attachments: [],
          memory: null,
          message_type: "text",
          metadata: null,
        },
      };
      return {
        ...state,
        messages: [...state.messages, newMessage],
        isStreaming: true,
        streamingContent: "",
      };
    }

    case "UPDATE_STREAMING_CONTENT": {
      // Find the last assistant text message and update its content
      const updatedMessages = [...state.messages];
      const lastAssistantIndex = [...updatedMessages]
        .reverse()
        .findIndex((m) => {
          const role = m.message.role.toLowerCase();
          const mType = (m.message.message_type || "").toLowerCase();
          return role === "assistant" && mType === "text";
        });

      if (lastAssistantIndex !== -1) {
        const actualIndex = updatedMessages.length - 1 - lastAssistantIndex;
        updatedMessages[actualIndex] = {
          ...updatedMessages[actualIndex],
          message: {
            ...updatedMessages[actualIndex].message,
            content: action.payload,
          },
        };
      }

      return {
        ...state,
        messages: updatedMessages,
        streamingContent: action.payload,
      };
    }

    case "FINALIZE_STREAM": {
      // Update the last assistant text message with final content
      const finalizedMessages = [...state.messages];
      const lastAssistIdx = [...finalizedMessages]
        .reverse()
        .findIndex((m) => {
          const role = m.message.role.toLowerCase();
          const mType = (m.message.message_type || "").toLowerCase();
          return role === "assistant" && mType === "text";
        });

      if (lastAssistIdx !== -1) {
        const actualIdx = finalizedMessages.length - 1 - lastAssistIdx;
        finalizedMessages[actualIdx] = {
          ...finalizedMessages[actualIdx],
          message: {
            ...finalizedMessages[actualIdx].message,
            content: action.payload,
          },
        };
      }

      return {
        ...state,
        messages: finalizedMessages,
        isStreaming: false,
        streamingContent: "",
        isLoading: false,
      };
    }

    case "ADD_ATTACHMENT_DATA":
      return {
        ...state,
        attachmentData: [...state.attachmentData, action.payload],
      };

    case "REMOVE_ATTACHMENT_DATA":
      return {
        ...state,
        attachmentData: state.attachmentData.filter(
          (_, idx) => idx !== action.payload,
        ),
      };

    case "CLEAR_ATTACHMENT_DATA":
      return {
        ...state,
        attachmentData: [],
      };

    case "ADD_ATTACHMENTS_TO_MESSAGE": {
      // Find message by ID and add attachments, don't duplicate
      const { messageId, attachments } = action.payload;

      const messagesWithAttachments = state.messages.map((msg) => {
        if (msg.message.id === messageId) {
          const existingIds = new Set(msg.message.attachments.map((a) => a.id));
          const filteredNew = attachments.filter((attachment) => {
            if (existingIds.has(attachment.id)) return false;
            existingIds.add(attachment.id);
            return true;
          });

          if (filteredNew.length === 0) return msg;

          return {
            ...msg,
            message: {
              ...msg.message,
              attachments: [...msg.message.attachments, ...filteredNew],
            },
          };
        }
        return msg;
      });

      return {
        ...state,
        messages: messagesWithAttachments,
      };
    }

    case "ATTACH_MEMORY": {
      // Find message by ID and attach memory to its message property
      const messagesWithMemory = state.messages.map((msg) => {
        if (msg.message.id === action.payload.messageId) {
          return {
            ...msg,
            message: {
              ...msg.message,
              memory: action.payload.memory,
            },
          };
        }
        return msg;
      });

      return {
        ...state,
        messages: messagesWithMemory,
      };
    }

    case "SET_OCR_TIMEOUT":
      return {
        ...state,
        ocrTimeoutRef: { current: action.payload },
      };

    case "CLEAR_OCR_TIMEOUT":
      if (state.ocrTimeoutRef.current) {
        clearTimeout(state.ocrTimeoutRef.current);
      }
      return {
        ...state,
        ocrTimeoutRef: { current: null },
      };

    case "CLEAR_MESSAGES":
      return {
        ...state,
        messages: [],
        conversationName: "",
        isStreaming: false,
        isLoading: false,
        streamingContent: "",
      };

    case "SET_LOADING":
      return {
        ...state,
        isLoading: action.payload,
      };

    case "SET_STREAMING":
      return {
        ...state,
        isStreaming: action.payload,
      };

    case "SET_OCR_LOADING":
      return {
        ...state,
        ocrLoading: action.payload,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface ConversationContextType {
  state: ConversationState;
  dispatch: React.Dispatch<ConversationAction>;
}

/**
 * Conversation Context
 */
const ConversationContext = createContext<ConversationContextType | undefined>(
  undefined,
);

/**
 * Conversation Provider Props
 */
interface ConversationProviderProps {
  children: ReactNode;
}

/**
 * Conversation Provider Component
 * Wraps the application to provide shared conversation state
 */
export function ConversationProvider({ children }: ConversationProviderProps) {
  const [state, dispatch] = useReducer(conversationReducer, initialState);

  // Use ref to track conversationId for event filtering without re-registering listeners
  const convIdRef = useRef(state.conversationId);
  useEffect(() => {
    convIdRef.current = state.conversationId;
  }, [state.conversationId]);

  // ============================================================
  // Event Listeners Setup - runs once when provider mounts
  // ============================================================
  useEffect(() => {
    let isMounted = true;
    const unlisteners: UnlistenFn[] = [];

    const setupEvents = async () => {
      if (!isMounted) return;

      try {
        console.log("[ConversationProvider] Setting up event listeners...");

        const listenerPromises = [
          // Stream Listener
          listen<ChatStreamEvent>("chat_stream", (event) => {
            const { delta, full_response, is_finished, conv_id } =
            event.payload;
            
            // Filter by conversation ID using ref
            if (!conv_id || (conv_id && conv_id !== convIdRef.current)) {
              return;
            }

            if (is_finished) {
              const finalText = extractThinkingContent(full_response);
              dispatch({ type: "FINALIZE_STREAM", payload: finalText });
              return;
            }

            if (delta) {
              const cleanContent = extractThinkingContent(full_response);
              dispatch({
                type: "UPDATE_STREAMING_CONTENT",
                payload: cleanContent,
              });
            }
          }),

          // Computer Use Listener
          listen<ComputerUseUpdateEvent>("computer_use_update", (event) => {
            const chatMessage: ChatMessage = {
              message: event.payload.message,
            };

            if (event.payload.status === "completed") {
              dispatch({
                type: "FINALIZE_STREAM",
                payload: event.payload.message.content,
              });
            } else {
              dispatch({ type: "ADD_CHAT_MESSAGE", payload: chatMessage });
            }
          }),

          // Memory Listener
          listen<MemoryExtractedEvent>("memory_extracted", (event) => {
            const { memory } = event.payload;

            if (memory.message_id) {
              dispatch({
                type: "ATTACH_MEMORY",
                payload: { messageId: memory.message_id, memory },
              });
            }
          }),

          // OCR Listener
          listen<OcrResponseEvent>("ocr_response", (event) => {
            if (event.payload.success && event.payload.text) {
              const ocrData: AttachmentData = {
                name: "Screen Capture",
                file_type: "ambient/ocr",
                data: event.payload.text,
              };
              dispatch({ type: "ADD_ATTACHMENT_DATA", payload: ocrData });
            }

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

          // Agentic Runtime Listeners
          listen<SkillActivatedEvent>("skill_activated", (event) => {
            const { skill_name, message_id, conversation_id, timestamp } =
              event.payload;

            if (conversation_id !== convIdRef.current) return;

            dispatch({
              type: "ADD_AGENTIC_MESSAGE",
              payload: {
                message: {
                  id: message_id,
                  conversation_id,
                  role: "assistant",
                  content: `Activated skill: ${skill_name}`,
                  timestamp,
                  message_type: "thinking",
                  metadata: {
                    type: "Thinking",
                    stage: `Skill Activated: ${skill_name}`,
                  },
                  attachments: [],
                  memory: null,
                },
              },
            });
          }),

          listen<ToolExecutionStartedEvent>(
            "tool_execution_started",
            (event) => {
              const {
                tool_call_id,
                message_id,
                skill_name,
                tool_name,
                arguments: args,
                timestamp,
              } = event.payload;

              dispatch({
                type: "ADD_AGENTIC_MESSAGE",
                payload: {
                  message: {
                    id: message_id,
                    conversation_id: convIdRef.current || "",
                    role: "assistant",
                    content: `Calling ${skill_name}.${tool_name}...`,
                    timestamp,
                    message_type: "tool_call",
                    metadata: {
                      type: "ToolCall",
                      call_id: tool_call_id,
                      skill_name,
                      tool_name,
                      arguments: args || {},
                    },
                    attachments: [],
                    memory: null,
                  },
                },
              });
            },
          ),

          listen<ToolExecutionCompletedEvent>(
            "tool_execution_completed",
            (event) => {
              const {
                tool_call_id,
                message_id,
                skill_name,
                tool_name,
                success,
                result,
                error,
                timestamp,
              } = event.payload;

              dispatch({
                type: "ADD_AGENTIC_MESSAGE",
                payload: {
                  message: {
                    id: message_id,
                    conversation_id: convIdRef.current || "",
                    role: "tool",
                    content: success
                      ? "Tool execution successful"
                      : `Tool error: ${error}`,
                    timestamp,
                    message_type: "tool_result",
                    metadata: {
                      type: "ToolResult",
                      call_id: tool_call_id,
                      success,
                      error: error || null,
                      result: result || null,
                    },
                    attachments: [],
                    memory: null,
                  },
                },
              });
            },
          ),
        ];

        const results = await Promise.all(listenerPromises);
        unlisteners.push(...results);

        console.log("[ConversationProvider] Event listeners initialized");
      } catch (error) {
        console.error("[ConversationProvider] Failed to setup events:", error);
      }
    };

    void setupEvents();

    // Cleanup on unmount
    return () => {
      isMounted = false;
      dispatch({ type: "CLEAR_OCR_TIMEOUT" });

      for (const unlisten of unlisteners) {
        try {
          unlisten();
        } catch (error) {
          console.error("[ConversationProvider] Error during cleanup:", error);
        }
      }
      console.log("[ConversationProvider] Event listeners cleaned up");
    };
  }, []);

  return (
    <ConversationContext.Provider value={{ state, dispatch }}>
      {children}
    </ConversationContext.Provider>
  );
}

/**
 * Hook to access conversation context
 * Must be used within a ConversationProvider
 */
export function useConversationContext(): ConversationContextType {
  const context = useContext(ConversationContext);

  if (!context) {
    throw new Error(
      "useConversationContext must be used within a ConversationProvider",
    );
  }

  return context;
}
