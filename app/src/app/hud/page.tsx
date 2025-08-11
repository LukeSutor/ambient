'use client';

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Move, GripVertical } from 'lucide-react';
import Image from "next/image";
const logo = '/logo.png';

interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export default function HudPage() {
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const streamContentRef = useRef<string>('');
  const [messages, setMessages] = useState<{ role: 'user' | 'assistant'; content: string }[]>([]);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const messagesContainerRef = useRef<HTMLDivElement | null>(null);

  // Helper to strip <think> blocks similar to landing page
  function extractThinkingContent(text: string) {
    const thinkStartIndex = text.indexOf('<think>');
    const thinkEndIndex = text.indexOf('</think>');
    let cleanText = text;
    if (thinkStartIndex !== -1) {
      if (thinkEndIndex !== -1) {
        cleanText = text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
      } else {
        cleanText = text.substring(0, thinkStartIndex);
      }
    }
    return cleanText;
  }

  // Initialize conversation, server, listener, and enable transparent background
  useEffect(() => {
    // Ensure LLM server is running and create a conversation
    (async () => {
      try {
        // Try spawning the llama.cpp server (idempotent if already running)
        await invoke<string>('spawn_llama_server');
      } catch (e) {
        // Not fatal; generate() does its own health check too
        console.warn('spawn_llama_server failed or not available:', e);
      }
      const convId = await createNewConversation();
      // Load any existing messages for this conversation
      if (convId) {
        try {
          const existing = await invoke<any[]>('get_messages', { conversationId: convId });
          const mapped = existing.map((m) => ({ role: m.role === 'user' ? 'user' as const : 'assistant' as const, content: extractThinkingContent(m.content) }));
          setMessages(mapped);
        } catch (err) {
          console.error('Failed to load existing messages for HUD:', err);
        }
      }
    })();

    // Set up a single stream listener mirroring the main UI approach
    let unlistenStream: UnlistenFn | null = null;
    (async () => {
      try {
        unlistenStream = await listen('chat-stream', (event) => {
          const payload: any = event.payload as any;
          const delta: string = payload?.delta ?? '';
          const isFinished: boolean = Boolean(payload?.is_finished);
          const full: string | undefined = payload?.full_response;

          if (isFinished) {
            const finalText = extractThinkingContent(full ?? streamContentRef.current);
            // Patch last assistant message
            setMessages((prev) => {
              const next = [...prev];
              const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
              const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
              if (lastIdx >= 0) {
                next[lastIdx] = { ...next[lastIdx], content: finalText };
              }
              return next;
            });
            setIsLoading(false);
            setIsStreaming(false);
            streamContentRef.current = '';
            return;
          }

          if (delta) {
            // Accumulate then render cleaned content
            streamContentRef.current += delta;
            const clean = extractThinkingContent(streamContentRef.current);
            setMessages((prev) => {
              const next = [...prev];
              const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
              const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
              if (lastIdx >= 0) {
                next[lastIdx] = { ...next[lastIdx], content: clean };
              } else {
                // If no assistant placeholder yet, add one
                next.push({ role: 'assistant', content: clean });
              }
              return next;
            });
            // Auto-scroll to bottom on new tokens
            queueMicrotask(() => {
              messagesEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
            });
          }
        });
      } catch (err) {
        console.error('Failed to set up chat-stream listener:', err);
      }
    })();

    // Add class to force transparent bg for this window
    if (typeof document !== 'undefined') {
      document.documentElement.classList.add('hud-transparent');
      document.body.classList.add('hud-transparent');
    }

    return () => {
      if (typeof document !== 'undefined') {
        document.documentElement.classList.remove('hud-transparent');
        document.body.classList.remove('hud-transparent');
      }
      if (unlistenStream) {
        try { unlistenStream(); } catch { }
      }
    };
  }, []);

  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>("create_conversation", { name: null });
      setCurrentConversationId(newConv.id);
      console.log('Created conversation:', newConv.id);
      return newConv.id;
    } catch (err) {
      console.error("Error creating conversation:", err);
      return null;
    }
  }

  async function closeWindow() {
    try {
      await invoke('close_floating_window', { label: 'floating-hud' });
    } catch (error) {
      console.error('Failed to close window:', error);
      try {
        const currentWindow = getCurrentWebviewWindow();
        await currentWindow.close();
      } catch (altError) {
        console.error('Direct close method also failed:', altError);
      }
    }
  }

  async function expandResponseArea() {
    setIsExpanded(true);
    try {
      const currentWindow = getCurrentWebviewWindow();
      const newSize = new LogicalSize(400, 350);
      await currentWindow.setSize(newSize);
    } catch (error) {
      console.error('Failed to resize window:', error);
    }
  }

  async function collapseResponseArea() {
    setIsExpanded(false);
    try {
      const currentWindow = getCurrentWebviewWindow();
      const newSize = new LogicalSize(400, 80);
      await currentWindow.setSize(newSize);
    } catch (error) {
      console.error('Failed to resize window:', error);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const query = input.trim();

    if (!query || isLoading) return;

    setIsLoading(true);
    setInput('');

    await expandResponseArea();

    try {
      // Append user message and an assistant placeholder
      setMessages((prev) => [...prev, { role: 'user', content: query }, { role: 'assistant', content: '' }]);
      setIsStreaming(true);
      streamContentRef.current = '';

      // Ensure we have a conversation id, create directly if missing to avoid race with state update
      let convId = currentConversationId;
      if (!convId) {
        try {
          const newConv = await invoke<Conversation>('create_conversation', { name: null });
          setCurrentConversationId(newConv.id);
          convId = newConv.id;
        } catch (convErr) {
          console.error('Failed to create conversation:', convErr);
        }
      }

      // Kick off generation with streaming always on and thinking disabled
      const finalText = await invoke<string>('generate', {
        prompt: query,
        jsonSchema: null,
        convId: convId ?? null,
        useThinking: false,
        stream: true,
      });

      // Safety: if no stream events arrive, stop loading when invoke resolves
      setIsLoading(false);
      setMessages((prev) => {
        // If stream produced content, leave it. Otherwise set finalText.
        const next = [...prev];
        const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
        const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
        if (lastIdx >= 0) {
          const existing = next[lastIdx]?.content ?? '';
          next[lastIdx] = {
            role: 'assistant',
            content: existing && existing.length > 0 ? existing : extractThinkingContent(finalText),
          };
        }
        return next;
      });
      setIsStreaming(false);
      streamContentRef.current = '';

    } catch (error) {
      console.error('Error generating response:', error);
      setMessages((prev) => [...prev.slice(0, -1), { role: 'assistant', content: '[Error generating response]' }]);
      setIsLoading(false);
      setIsStreaming(false);
      streamContentRef.current = '';
    }
  }

  async function clearAndCollapse() {
    setMessages([]);
    await collapseResponseArea();
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e as any);
    }
    if (e.key === 'Escape') {
      clearAndCollapse();
    }
  };

  return (
    <div className="w-full h-full bg-transparent">
      {/* Glass Container */}
      <div className="relative w-full h-full rounded-2xl backdrop-blur-2xl backdrop-saturate-150 ring-1 ring-black/10 shadow-[0_8px_30px_rgba(0,0,0,0.08)] flex flex-col overflow-hidden">
        {/* Subtle sheen and vignette overlays for depth (light glass) */}
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-white/50 via-transparent to-transparent" />
        <div className="pointer-events-none absolute inset-0 [mask-image:radial-gradient(80%_80%_at_50%_30%,black,transparent)] shadow-[inset_0_-40px_80px_rgba(0,0,0,0.10)]" />

        {/* Chat Area + Input pinned to bottom */}
        <div className="relative flex-1 flex flex-col min-h-0">
          {/* Messages Scroll Area */}
          {isExpanded && (
            <div
              ref={messagesContainerRef}
              className="hud-scroll flex-1 min-h-0 overflow-y-auto p-3 space-y-2 text-black/90 text-sm leading-relaxed bg-white/30 transition-all"
            >
              {messages.map((m, i) => (
                <div
                  key={`m-${i}`}
                  className={
                    m.role === 'user'
                      ? 'max-w-[85%] ml-auto bg-white/60 border border-black/10 rounded-xl px-3 py-2'
                      : 'max-w-[95%] mx-auto px-3 py-2'
                  }
                >
                  <div className="whitespace-pre-wrap">{m.content}</div>
                </div>
              ))}
              {/* streaming cursor */}
              {isStreaming && (
                <div className="mr-auto h-4 w-3 animate-pulse rounded-sm bg-black/30" />
              )}
              <div ref={messagesEndRef} />
            </div>
          )}

          {/* Input Container pinned bottom; animates placement via margin */}
          <div
            className={
              'flex items-center gap-3 h-[55px] p-2 flex-shrink-0 border-t border-black/10 transition-all ' +
              (isExpanded ? 'mt-auto' : '')
            }
          >
            <Image src={logo} width={32} height={32} alt="Logo" className="w-8 h-8 rounded-full bg-white/80 p-1" />

            <div className="flex-1">
              <Input
                type="text"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Ask me anything..."
                className="bg-white/50 border-black/15 rounded-xl text-black placeholder:text-black/50 focus:bg-white/40 focus:border-black/30 focus:shadow-[0_0_0_1px_rgba(0,0,0,0.08),0_0_20px_rgba(0,0,0,0.08)] backdrop-blur-sm transition-all"
                autoComplete="off"
                autoFocus
              />
            </div>

            {/* Drag Handle */}
            <div
              data-tauri-drag-region
              className="w-9 h-9 border rounded-full border-black/20 flex items-center justify-center cursor-grab bg-white/20 hover:bg-white/40 hover:border-black/20 backdrop-blur-sm transition-all"
              title="Drag Window"
            >
              <Move data-tauri-drag-region className="w-6 h-6 text-black" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}