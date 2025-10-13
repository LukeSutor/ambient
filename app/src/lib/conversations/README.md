# Conversation Management Library

A simplified, extensible conversation management system for Next.js + Tauri applications.

## Architecture

This library provides a clean pattern for managing real-time conversations with streaming responses and memory attachments.

### Core Components

```
lib/conversations/
  ├── ConversationProvider.tsx  # Context provider with reducer
  ├── useConversation.ts        # Main hook with all functionality
  ├── api.ts                    # Tauri backend API calls
  ├── types.ts                  # TypeScript interfaces
  └── index.ts                  # Public exports
```

## Usage

### 1. Wrap your app with the provider

```tsx
import { ConversationProvider } from '@/lib/conversations';

export default function Layout({ children }) {
  return (
    <ConversationProvider>
      {children}
    </ConversationProvider>
  );
}
```

### 2. Use the hook in your components

```tsx
import { useConversation } from '@/lib/conversations';

export default function ChatPage() {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  
  const {
    // State
    conversationId,
    messages,
    isStreaming,
    isLoading,
    
    // Operations
    sendMessage,
    createNew,
    loadMessages,
    clear,
    ensureServer,
  } = useConversation(messagesEndRef); // Optional ref for auto-scroll

  const handleSend = async () => {
    await sendMessage(conversationId!, 'Hello!', []);
  };

  return (
    <div>
      {messages.map(msg => (
        <div key={msg.id}>{msg.content}</div>
      ))}
      <div ref={messagesEndRef} />
    </div>
  );
}
```

## Features

### ✅ Real-time Streaming
- Automatic handling of Tauri `chat_stream` events
- Progressive content updates with thinking tag removal
- Auto-scroll support via optional ref

### ✅ Memory Attachments
- Listens for `memory_extracted` events
- Automatically attaches memories to messages
- Supports user message memory context

### ✅ Conversation Safety
- ConversationId filtering prevents state corruption
- Safe to switch conversations mid-stream
- Events for wrong conversations are ignored

### ✅ Clean API
- Single hook for all functionality
- No action creators or boilerplate
- Inline event handlers with proper cleanup

## API Reference

### State

| Property | Type | Description |
|----------|------|-------------|
| `conversationId` | `string \| null` | Current active conversation ID |
| `messages` | `ChatMessage[]` | Array of all messages |
| `isStreaming` | `boolean` | Whether AI is currently streaming |
| `isLoading` | `boolean` | Whether a request is in progress |
| `streamingContent` | `string` | Current streaming content buffer |

### Operations

#### `sendMessage(conversationId, content, ocrResults?)`
Sends a message to the conversation with optional OCR context.

```typescript
await sendMessage(conversationId, 'What is this?', ocrResults);
```

#### `createNew(name?)`
Creates a new conversation and switches to it.

```typescript
const newId = await createNew('My Conversation');
```

#### `loadMessages(conversationId)`
Loads message history for a specific conversation.

```typescript
await loadMessages(conversationId);
```

#### `clear()`
Clears all messages from the current conversation (local only).

```typescript
clear();
```

#### `ensureServer()`
Ensures the Llama server is running.

```typescript
await ensureServer();
```

## Types

### ChatMessage
```typescript
interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  memory: MemoryEntry | null;
  timestamp: string;
}
```

### Conversation
```typescript
interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}
```

## Extension Pattern

This library serves as a blueprint for similar feature management systems. To create a new feature (e.g., tasks, settings):

1. **Create a Provider** - Manages shared state with `useReducer`
2. **Create a Hook** - Combines state, events, and operations
3. **Keep it Simple** - Inline helpers, direct dispatches, minimal abstraction
4. **Filter Events** - Always filter Tauri events by relevant IDs

### Example: Task Management

```typescript
// TaskProvider.tsx
export function TaskProvider({ children }) {
  const [state, dispatch] = useReducer(taskReducer, initialState);
  return <TaskContext.Provider value={{ state, dispatch }}>{children}</TaskContext.Provider>;
}

// useTask.ts
export function useTask() {
  const { state, dispatch } = useTaskContext();
  
  // Set up event listeners
  useEffect(() => {
    const unlisten = listen('task_updated', (event) => {
      if (event.payload.task_id === state.taskId) {
        dispatch({ type: 'UPDATE_TASK', payload: event.payload });
      }
    });
    return () => unlisten();
  }, [state.taskId]);
  
  // Operations
  const createTask = async (name) => { /* ... */ };
  
  return { ...state, createTask };
}
```

## Design Principles

1. **Single Source of Truth** - Context provider ensures shared state
2. **Minimal Abstraction** - No unnecessary layers or wrappers
3. **Event Safety** - Always filter events by ID
4. **Easy to Understand** - All logic in 2-3 files
5. **Extensible** - Clear pattern for future features

## Migration Notes

If migrating from the old system:
- `useConversationManager` → `useConversation`
- All functionality is the same, just simplified internally
- No breaking changes to the public API
