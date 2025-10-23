import { RefObject } from "react";

/**
 * Windows state
 */
export interface WindowsState {
    isLogin: boolean;
    isChatExpanded: boolean;
    isFeaturesExpanded: boolean;
    settingsDestination: string;
    messagesContainerRef: RefObject<HTMLDivElement | null>;
    featuresRef: RefObject<HTMLDivElement | null>;
}
