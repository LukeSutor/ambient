'use client';

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Move, X } from 'lucide-react';

interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export default function HudPage() {
  const [input, setInput] = useState('');
  const [response, setResponse] = useState('');
  const [userQuery, setUserQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);

  // Initialize conversation on mount and enable transparent background
  useEffect(() => {
    createNewConversation();

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
    };
  }, []);

  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>("create_conversation", { name: null });
      setCurrentConversationId(newConv.id);
      console.log('Created conversation:', newConv.id);
    } catch (err) {
      console.error("Error creating conversation:", err);
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
    setUserQuery(query);
    setInput('');
    
    await expandResponseArea();
    
    // Create conversation if needed
    if (!currentConversationId) {
      await createNewConversation();
    }
    
    try {
      // Simulate response for now - will connect to real LLM later
      setResponse('');
      setTimeout(() => {
        setResponse(`This is a simulated response to: "${query}"\n\nThis is now a React component with TypeScript and Tailwind!`);
        setIsLoading(false);
      }, 1500);
      
    } catch (error) {
      console.error('Error generating response:', error);
      setResponse('Sorry, there was an error processing your request.');
      setIsLoading(false);
    }
  }

  async function clearAndCollapse() {
    setUserQuery('');
    setResponse('');
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
      <div className="w-full h-full bg-transparent">
        {/* Glass Container */}
        <div className="relative w-full h-full rounded-xl bg-gradient-to-br from-white/25 via-white/10 to-white/5 backdrop-blur-xl backdrop-saturate-150 border-0 shadow-[inset_0_1px_0_rgba(255,255,255,0.4),inset_0_-1px_0_rgba(255,255,255,0.1)] flex flex-col">
        
        {/* Close Button */}
        <Button
          variant="ghost"
          size="sm"
          onClick={closeWindow}
          className="absolute top-2 right-2 w-6 h-6 p-0 rounded-full bg-white/15 border border-white/20 text-white/80 hover:bg-white/25 hover:border-white/40 hover:scale-110 transition-all z-10"
        >
          <X className="w-3 h-3" />
        </Button>

        {/* Input Container */}
        <div className="flex items-center p-3 gap-3 flex-shrink-0 bg-white/5 border-b border-white/10">
          <div className="flex-1">
            <Input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask me anything..."
              className="bg-white/15 border-white/20 rounded-xl text-white placeholder:text-white/60 focus:bg-white/20 focus:border-white/40 focus:shadow-[0_0_20px_rgba(255,255,255,0.1)] backdrop-blur-sm transition-all"
              autoComplete="off"
              autoFocus
            />
          </div>
          
          {/* Drag Handle */}
          <div
            data-tauri-drag-region
            className="w-9 h-9 rounded-full bg-white/20 border border-white/30 flex items-center justify-center cursor-grab hover:bg-white/25 hover:border-white/40 hover:scale-105 active:scale-95 backdrop-blur-sm transition-all"
          >
            <Move className="w-4 h-4 text-white/80" />
          </div>
        </div>

        {/* Response Container */}
        {isExpanded && (
          <div className="flex-1 overflow-y-auto">
            <div className="p-4 text-white/90 text-sm leading-relaxed bg-black/10">
              {userQuery && (
                <div className="bg-white/10 border border-white/15 rounded-lg p-3 mb-3 font-medium">
                  {userQuery}
                </div>
              )}
              
              <div className="whitespace-pre-wrap">
                {isLoading ? (
                  <div className="flex items-center gap-1">
                    <div className="flex gap-1">
                      <div className="w-1 h-1 bg-white/60 rounded-full animate-bounce [animation-delay:-0.32s]"></div>
                      <div className="w-1 h-1 bg-white/60 rounded-full animate-bounce [animation-delay:-0.16s]"></div>
                      <div className="w-1 h-1 bg-white/60 rounded-full animate-bounce"></div>
                    </div>
                  </div>
                ) : (
                  response
                )}
              </div>
            </div>
          </div>
        )}
        </div>
      </div>
    </div>
  );
}