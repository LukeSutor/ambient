import { ConversationList } from "@/components/hud/conversation-list";
import { MessageList } from "@/components/hud/message-list";
import { useConversation } from "@/lib/conversations";
import { useSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { useWindows } from "@/lib/windows/useWindows";
import type React from "react";
import { useMemo } from "react";

export function DynamicChatContent() {
  const { isChatExpanded, isChatHistoryExpanded } = useWindows();
  const { hudDimensions } = useSettings();

  const isVisible = isChatExpanded || isChatHistoryExpanded;
  const showBothPanels = isChatExpanded && isChatHistoryExpanded;
  const maxHeight = hudDimensions
    ? `${hudDimensions.chat_max_height}px`
    : "500px";

  // Memoized class computations
  const conversationsClass = useMemo(() => {
    if (!isChatHistoryExpanded)
      return "w-0 max-h-0 opacity-0 pointer-events-none";
    if (!isChatExpanded) return "w-full max-h-96 opacity-100";
    return "w-[60%] min-h-32 max-h-full opacity-100";
  }, [isChatExpanded, isChatHistoryExpanded]);

  const messagesClass = useMemo(() => {
    if (!isChatExpanded) return "w-0 max-h-0 opacity-0 pointer-events-none";
    return "w-full max-h-full opacity-100";
  }, [isChatExpanded]);

  const containerStyle = useMemo<React.CSSProperties>(
    () => ({ maxHeight: isVisible ? maxHeight : "0px" }),
    [isVisible, maxHeight],
  );

  return (
    <div
      className={cn(
        "flex flex-col transition-[max-height,opacity] duration-300 ease-in-out overflow-hidden",
        isVisible ? "opacity-100 pb-2" : "opacity-0 pointer-events-none",
      )}
      style={containerStyle}
    >
      <div
        className={cn(
          "flex flex-row justify-center min-h-0",
          showBothPanels && "space-x-2",
        )}
      >
        {/* Conversation list */}
        <div
          className={cn(
            "overflow-hidden transition-all duration-300 min-h-0",
            conversationsClass,
          )}
        >
          <ConversationList />
        </div>

        {/* Message list */}
        <div
          className={cn(
            "overflow-hidden transition-all duration-300 min-h-0",
            messagesClass,
          )}
        >
          <MessageList />
        </div>
      </div>
    </div>
  );
}
