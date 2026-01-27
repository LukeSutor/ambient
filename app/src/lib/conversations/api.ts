import type { Conversation } from "@/types/conversations";
import {
  type AttachmentData,
  type GenerateConversationNameEvent,
  type HudChatEvent,
  OcrResponseEvent,
} from "@/types/events";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";

/**
 * Creates a new conversation
 * @param name - Optional conversation name
 * @returns Promise resolving to the created Conversation
 */
export async function createConversation(
  name?: string,
  convType?: string | null,
): Promise<Conversation> {
  try {
    const conversation = await invoke<Conversation>("create_conversation", {
      name: name || null,
      convType: convType || null,
    });
    return conversation;
  } catch (error) {
    console.error("[ConversationAPI] Failed to create conversation:", error);
    throw new Error("Failed to create conversation");
  }
}

/**
 * Sends a message and triggers LLM generation with streaming
 * @param conversationId - ID of the conversation
 * @param content - Message content
 * @param ocrResults - OCR context to include with the message
 * @param messageId - The message ID to use for the user message
 * @returns Promise resolving to final response text (may not be needed if streaming)
 */
export async function sendMessage(
  conversationId: string,
  content: string,
  attachmentData: AttachmentData[],
  messageId: string,
): Promise<string> {
  try {
    const hudChatEvent: HudChatEvent = {
      text: content,
      conv_id: conversationId,
      timestamp: Date.now().toString(),
      message_id: messageId,
      attachments: attachmentData,
    };

    const finalText = await invoke<string>("handle_hud_chat", {
      event: hudChatEvent,
    });

    return finalText;
  } catch (error) {
    console.error("[ConversationAPI] Failed to send message:", error);
    throw new Error("Failed to send message");
  }
}

/**
 * Starts a computer use session
 * @param conversationId - ID of the conversation
 * @param prompt - The prompt to initiate computer use
 */
export async function startComputerUseSession(
  conversationId: string,
  prompt: string,
): Promise<void> {
  try {
    await invoke("start_computer_use", { conversationId, prompt });
  } catch (error) {
    console.error(
      "[ConversationAPI] Failed to start computer use session:",
      error,
    );
    throw new Error("Failed to start computer use session");
  }
}

/**
 * Stops the current computer use session
 */
export async function stopComputerUseSession(): Promise<void> {
  try {
    await invoke("stop_computer_use");
  } catch (error) {
    console.error(
      "[ConversationAPI] Failed to stop computer use session:",
      error,
    );
    throw new Error("Failed to stop computer use session");
  }
}

/**
 * Emits an event to generate a conversation name
 * @param conversationId - ID of the conversation
 * @param message - The message content to base the name on
 */
export async function emitGenerateConversationName(
  conversationId: string,
  message: string,
): Promise<void> {
  const generateConversationNameEvent: GenerateConversationNameEvent = {
    conv_id: conversationId,
    message: message,
    timestamp: Date.now().toString(),
  };

  try {
    await emit("generate_conversation_name", generateConversationNameEvent);
  } catch (error) {
    console.error(
      "[ConversationAPI] Failed to generate conversation name:",
      error,
    );
    throw new Error("Failed to generate conversation name");
  }
}

/**
 * Deletes a conversation and all its messages
 * @param conversationId - ID of the conversation to delete
 */
export async function deleteConversation(
  conversationId: string,
): Promise<void> {
  try {
    await invoke("delete_conversation", { conversationId });
  } catch (error) {
    console.error("[ConversationAPI] Failed to delete conversation:", error);
    throw new Error("Failed to delete conversation");
  }
}

/**
 * Ensures the llama server is running
 */
export async function ensureLlamaServerRunning(): Promise<void> {
  try {
    await invoke<string>("spawn_llama_server");
  } catch (error) {
    console.warn("[ConversationAPI] spawn_llama_server warning:", error);
    // Don't throw - this is not critical
  }
}
