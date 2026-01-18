"use client";

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useReducer,
  useRef,
  type ReactNode,
} from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  invokeGetFullAuthState,
} from "./commands";
import type { UserInfo, RoleAccessState } from "./types";

const initialState: RoleAccessState = {
  isHydrated: false,
  isOnline: false,
  isLoggedIn: false,
  isSetupComplete: false,
  isPremiumUser: false,
  userInfo: null,
};

type RoleAccessAction =
  | { type: "SET_IS_ONLINE"; payload: boolean }
  | { type: "SET_LOGGED_IN"; payload: boolean }
  | { type: "SET_SETUP_COMPLETE"; payload: boolean }
  | { type: "SET_PREMIUM_USER"; payload: boolean }
  | { type: "SET_USER_INFO"; payload: UserInfo | null }
  | { type: "SET_IS_HYDRATED"; payload: boolean }
  | { type: "SET_FULL_STATE"; payload: { isOnline: boolean; isLoggedIn: boolean; isSetupComplete: boolean; userInfo: UserInfo | null } };


function roleAccessReducer(state: RoleAccessState, action: RoleAccessAction): RoleAccessState {
  switch (action.type) {
    case "SET_IS_ONLINE":
      return {
        ...state,
        isOnline: action.payload,
      };
    case "SET_LOGGED_IN":
      return {
        ...state,
        isLoggedIn: action.payload,
      };
    case "SET_SETUP_COMPLETE":
      return {
        ...state,
        isSetupComplete: action.payload,
      };
    case "SET_PREMIUM_USER":
      return {
        ...state,
        isPremiumUser: action.payload,
      };
    case "SET_USER_INFO":
      return {
        ...state,
        userInfo: action.payload,
      };
    case "SET_IS_HYDRATED":
      return {
        ...state,
        isHydrated: action.payload,
      };
    case "SET_FULL_STATE":
      return {
        ...state,
        isOnline: action.payload.isOnline,
        isLoggedIn: action.payload.isLoggedIn,
        isSetupComplete: action.payload.isSetupComplete,
        userInfo: action.payload.userInfo,
      };
    default:
      return state;
  }
}

interface RoleAccessContextType {
  state: RoleAccessState;
  dispatch: React.Dispatch<RoleAccessAction>;
  refresh: () => Promise<void>;
}

const RoleAccessContext = createContext<RoleAccessContextType | undefined>(undefined);

interface RoleAccessProviderProps {
  children: ReactNode;
}

export function RoleAccessProvider({ children }: RoleAccessProviderProps) {
  const [state, dispatch] = useReducer(roleAccessReducer, initialState);
  const isRefreshing = useRef(false);

  const refresh = useCallback(async () => {
    if (isRefreshing.current) return;
    console.log("Refreshing role access state...");
    
    isRefreshing.current = true;
    try {
      const fullState = await invokeGetFullAuthState();
      
      dispatch({ 
        type: "SET_FULL_STATE", 
        payload: {
          isOnline: fullState.is_online,
          isLoggedIn: fullState.is_authenticated,
          isSetupComplete: fullState.is_setup_complete,
          userInfo: fullState.user,
        }
      });
    } catch (error) {
      console.error("Error fetching role access state:", error);
    } finally {
      isRefreshing.current = false;
      dispatch({ type: "SET_IS_HYDRATED", payload: true });
    }
  }, [dispatch]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    void refresh();

    const subscribe = async () => {
      try {
        unlisten = await listen("auth_changed", () => {
          void refresh();
        });
      } catch (error) {
        console.error("Failed to subscribe to auth_changed event:", error);
      }
    };

    void subscribe();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [refresh]);

  return (
    <RoleAccessContext.Provider value={{ state, dispatch, refresh }}>
      {children}
    </RoleAccessContext.Provider>
  );
}

export function useRoleAccessContext(): RoleAccessContextType {
  const context = useContext(RoleAccessContext);

  if (!context) {
    throw new Error("useRoleAccessContext must be used within a RoleAccessProvider");
  }

  return context;
}
