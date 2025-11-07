"use client";

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useReducer,
  type ReactNode,
} from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  invokeGetCurrentUser,
  invokeIsAuthenticated,
  invokeIsSetupComplete,
} from "./commands";
import type { CognitoUserInfo, RoleAccessState } from "./types";

const initialState: RoleAccessState = {
  isHydrated: false,
  isLoggedIn: false,
  isSetupComplete: false,
  isPremiumUser: false,
  userInfo: null,
};

type RoleAccessAction =
  | { type: "SET_LOGGED_IN"; payload: boolean }
  | { type: "SET_SETUP_COMPLETE"; payload: boolean }
  | { type: "SET_PREMIUM_USER"; payload: boolean }
  | { type: "SET_USER_INFO"; payload: CognitoUserInfo | null }
  | { type: "SET_IS_HYDRATED"; payload: boolean };

function roleAccessReducer(state: RoleAccessState, action: RoleAccessAction): RoleAccessState {
  switch (action.type) {
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

  const refresh = useCallback(async () => {
    try {
      const [isLoggedIn, isSetupComplete] = await Promise.all([
        invokeIsAuthenticated(),
        invokeIsSetupComplete(),
      ]);

      console.log({ isLoggedIn, isSetupComplete });

      dispatch({ type: "SET_LOGGED_IN", payload: isLoggedIn });
      dispatch({ type: "SET_SETUP_COMPLETE", payload: isSetupComplete });

      if (isLoggedIn) {
        const userInfo = await invokeGetCurrentUser();
        dispatch({ type: "SET_USER_INFO", payload: userInfo });
      } else {
        dispatch({ type: "SET_USER_INFO", payload: null });
      }
    } catch (error) {
      console.error("Error fetching role access state:", error);
    } finally {
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
