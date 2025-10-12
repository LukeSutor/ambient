import { ChatMessage, MessageRole } from './types';
import { MemoryEntry } from '@/types/memory';

/**
 * Extracts and removes <think> tags from LLM responses
 * @param text - Raw text from LLM that may contain thinking blocks
 * @returns Cleaned text without thinking blocks
 */
export function extractThinkingContent(text: string): string {
  const thinkStartIndex = text.indexOf('<think>');
  const thinkEndIndex = text.indexOf('</think>');
  
  let cleanText = text;
  
  if (thinkStartIndex !== -1) {
    if (thinkEndIndex !== -1) {
      // Remove complete thinking block
      cleanText = text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
    } else {
      // Remove incomplete thinking block
      cleanText = text.substring(0, thinkStartIndex);
    }
  }
  
  return cleanText;
}

/**
 * Transforms a backend message format to frontend ChatMessage format
 * @param backendMessage - Message from Tauri backend
 * @returns Normalized ChatMessage
 */
export function transformBackendMessage(backendMessage: any): ChatMessage {
  return {
    id: backendMessage.id,
    role: backendMessage.role === 'user' ? 'user' : 'assistant',
    content: extractThinkingContent(backendMessage.content),
    memory: backendMessage.memory ? (backendMessage.memory as MemoryEntry) : null,
    timestamp: backendMessage.timestamp,
  };
}

/**
 * Creates a new user message
 * @param content - Message content
 * @param memory - Optional memory entry to attach
 * @returns New ChatMessage
 */
export function createUserMessage(content: string, memory: MemoryEntry | null = null): ChatMessage {
  return {
    id: crypto.randomUUID(),
    role: 'user',
    content,
    memory,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Creates a new assistant message (typically empty for streaming)
 * @param content - Initial content (usually empty string)
 * @returns New ChatMessage
 */
export function createAssistantMessage(content: string = ''): ChatMessage {
  return {
    id: crypto.randomUUID(),
    role: 'assistant',
    content,
    memory: null,
    timestamp: new Date().toISOString(),
  };
}
