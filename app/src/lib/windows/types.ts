import { RefObject } from "react";

/**
 * Windows state
 */
export interface WindowsState {
    isLogin: boolean;
    isChatExpanded: boolean;
    isFeaturesExpanded: boolean;
    isChatHistoryExpanded: boolean;
    settingsDestination: string;
    dynamicChatContentRef: RefObject<HTMLDivElement | null>;
    featuresRef: RefObject<HTMLDivElement | null>;
    resizeObserverRef: RefObject<ResizeObserver | null>;
}
