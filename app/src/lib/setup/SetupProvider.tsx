"use client";

import type {
  DownloadFinishedEvent,
  DownloadInformationEvent,
  DownloadProgressEvent,
  DownloadStartedEvent,
} from "@/types/events";
import { invoke } from "@tauri-apps/api/core";
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

/**
 * Setup state
 */
interface SetupState {
  isDownloading: boolean;
  numModels: number;
  totalContentLength: number;
  downloadedBytes: number[]; // Stores the downloaded bytes for each model
  downloadingId: number | null;
}

/**
 * Initial state
 */
const initialState: SetupState = {
  isDownloading: false,
  numModels: 0,
  totalContentLength: -1,
  downloadedBytes: [],
  downloadingId: null,
};

/**
 * Action types
 */
type SetupAction =
  | { type: "SET_IS_DOWNLOADING"; payload: boolean }
  | { type: "SET_N_MODELS"; payload: number }
  | { type: "SET_TOTAL_CONTENT_LENGTH"; payload: number }
  | {
      type: "SET_DOWNLOADED_BYTES";
      payload: { model_id: number; bytes: number };
    }
  | { type: "SET_DOWNLOADING_ID"; payload: number | null };

/**
 * Setup reducer
 */
function setupReducer(state: SetupState, action: SetupAction): SetupState {
  switch (action.type) {
    case "SET_IS_DOWNLOADING":
      return {
        ...state,
        isDownloading: action.payload,
      };

    case "SET_N_MODELS":
      return {
        ...state,
        numModels: action.payload,
      };

    case "SET_TOTAL_CONTENT_LENGTH":
      return {
        ...state,
        totalContentLength: action.payload,
      };

    case "SET_DOWNLOADED_BYTES":
      return {
        ...state,
        downloadedBytes: [
          ...state.downloadedBytes.slice(0, action.payload.model_id),
          action.payload.bytes,
          ...state.downloadedBytes.slice(action.payload.model_id + 1),
        ],
      };

    case "SET_DOWNLOADING_ID":
      return {
        ...state,
        downloadingId: action.payload,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface SetupContextType {
  state: SetupState;
  dispatch: React.Dispatch<SetupAction>;
}

/**
 * Setup Context
 */
const SetupContext = createContext<SetupContextType | undefined>(undefined);

/**
 * Setup Provider Props
 */
interface SetupProviderProps {
  children: ReactNode;
}

/**
 * Setup Provider Component
 * Provides shared setup state across the application
 */
export function SetupProvider({ children }: SetupProviderProps) {
  const [state, dispatch] = useReducer(setupReducer, initialState);

  // Use a ref to keep track of the latest state in async callbacks
  const stateRef = useRef(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // Event listeners setup
  useEffect(() => {
    let isMounted = true;
    const unlisteners: UnlistenFn[] = [];

    const setupEvents = async () => {
      if (!isMounted) return;

      try {
        console.log("[SetupProvider] Setting up event listeners...");

        const listenerPromises = [
          listen<DownloadInformationEvent>("download_information", (event) => {
            console.log({ event });
            dispatch({
              type: "SET_N_MODELS",
              payload: Number(event.payload.n_items),
            });
            dispatch({
              type: "SET_TOTAL_CONTENT_LENGTH",
              payload: Number(event.payload.content_length),
            });
          }),
          listen<DownloadStartedEvent>("download_started", (event) => {
            console.log({ event });
            dispatch({ type: "SET_IS_DOWNLOADING", payload: true });
            dispatch({
              type: "SET_DOWNLOADING_ID",
              payload: Number(event.payload.id),
            });
          }),
          listen<DownloadProgressEvent>("download_progress", (event) => {
            dispatch({
              type: "SET_DOWNLOADED_BYTES",
              payload: {
                model_id: Number(event.payload.id),
                bytes: Number(event.payload.total_progress),
              },
            });
          }),
          listen<DownloadFinishedEvent>("download_finished", (event) => {
            console.log({ event });
            dispatch({ type: "SET_DOWNLOADING_ID", payload: null });
            if (Number(event.payload.id) === stateRef.current.numModels) {
              dispatch({ type: "SET_IS_DOWNLOADING", payload: false });
            }
          }),
        ];

        const results = await Promise.all(listenerPromises);
        unlisteners.push(...results);

        console.log("[SetupProvider] Event listeners initialized");
      } catch (error) {
        console.error("[SetupProvider] Failed to setup events:", error);
      }
    };

    // Fetch total bytes needed and number of items for download
    const fetchDownloadInfo = async () => {
      try {
        const info = await invoke<DownloadInformationEvent>(
          "get_setup_download_info",
        );
        console.log({ info });
        dispatch({ type: "SET_N_MODELS", payload: Number(info.n_items) });
        dispatch({
          type: "SET_TOTAL_CONTENT_LENGTH",
          payload: Number(info.content_length),
        });
      } catch (error) {
        console.error(
          "[SetupProvider] Failed to fetch download information:",
          error,
        );
      }
    };

    void fetchDownloadInfo();
    void setupEvents();

    return () => {
      isMounted = false;
      for (const unlisten of unlisteners) {
        try {
          unlisten();
        } catch (error) {
          console.error("[SetupProvider] Error during cleanup:", error);
        }
      }
      console.log("[SetupProvider] Event listeners cleaned up");
    };
  }, []);

  return (
    <SetupContext.Provider value={{ state, dispatch }}>
      {children}
    </SetupContext.Provider>
  );
}

/**
 * Hook to access setup context
 * Must be used within a SetupProvider
 */
export function useSetupContext(): SetupContextType {
  const context = useContext(SetupContext);

  if (!context) {
    throw new Error("useSetupContext must be used within a SetupProvider");
  }

  return context;
}
