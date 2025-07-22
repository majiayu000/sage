/**
 * App state context for managing global application state
 */

import React, { createContext, useContext, useReducer, ReactNode } from 'react';
import { SageConfig, CliArgs, ConversationMessage, TaskMetadata } from '../types/config.js';

interface AppState {
  config: SageConfig;
  args: CliArgs;
  messages: ConversationMessage[];
  currentTask: TaskMetadata | null;
  isProcessing: boolean;
  error: string | null;
  status: string;
}

type AppAction =
  | { type: 'SET_CONFIG'; payload: SageConfig }
  | { type: 'ADD_MESSAGE'; payload: ConversationMessage }
  | { type: 'CLEAR_MESSAGES' }
  | { type: 'SET_CURRENT_TASK'; payload: TaskMetadata | null }
  | { type: 'SET_PROCESSING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null }
  | { type: 'SET_STATUS'; payload: string }
  | { type: 'RESET_STATE' };

const initialState: AppState = {
  config: {} as SageConfig,
  args: {} as CliArgs,
  messages: [],
  currentTask: null,
  isProcessing: false,
  error: null,
  status: 'Ready',
};

function appStateReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case 'SET_CONFIG':
      return { ...state, config: action.payload };
    
    case 'ADD_MESSAGE':
      return { 
        ...state, 
        messages: [...state.messages, action.payload],
        error: null 
      };
    
    case 'CLEAR_MESSAGES':
      return { ...state, messages: [] };
    
    case 'SET_CURRENT_TASK':
      return { ...state, currentTask: action.payload };
    
    case 'SET_PROCESSING':
      return { 
        ...state, 
        isProcessing: action.payload,
        status: action.payload ? 'Processing...' : 'Ready'
      };
    
    case 'SET_ERROR':
      return { 
        ...state, 
        error: action.payload,
        isProcessing: false,
        status: action.payload ? 'Error' : 'Ready'
      };
    
    case 'SET_STATUS':
      return { ...state, status: action.payload };
    
    case 'RESET_STATE':
      return { 
        ...initialState, 
        config: state.config, 
        args: state.args 
      };
    
    default:
      return state;
  }
}

const AppStateContext = createContext<{
  state: AppState;
  dispatch: React.Dispatch<AppAction>;
} | null>(null);

interface AppStateProviderProps {
  config: SageConfig;
  args: CliArgs;
  children: ReactNode;
}

export const AppStateProvider: React.FC<AppStateProviderProps> = ({ 
  config, 
  args, 
  children 
}) => {
  const [state, dispatch] = useReducer(appStateReducer, {
    ...initialState,
    config,
    args,
  });

  return (
    <AppStateContext.Provider value={{ state, dispatch }}>
      {children}
    </AppStateContext.Provider>
  );
};

export const useAppState = () => {
  const context = useContext(AppStateContext);
  if (!context) {
    throw new Error('useAppState must be used within an AppStateProvider');
  }
  return context;
};

// Convenience hooks for specific state slices
export const useMessages = () => {
  const { state, dispatch } = useAppState();
  
  const addMessage = (message: ConversationMessage) => {
    dispatch({ type: 'ADD_MESSAGE', payload: message });
  };
  
  const clearMessages = () => {
    dispatch({ type: 'CLEAR_MESSAGES' });
  };
  
  return {
    messages: state.messages,
    addMessage,
    clearMessages,
  };
};

export const useProcessing = () => {
  const { state, dispatch } = useAppState();
  
  const setProcessing = (isProcessing: boolean) => {
    dispatch({ type: 'SET_PROCESSING', payload: isProcessing });
  };
  
  return {
    isProcessing: state.isProcessing,
    setProcessing,
  };
};

export const useError = () => {
  const { state, dispatch } = useAppState();
  
  const setError = (error: string | null) => {
    dispatch({ type: 'SET_ERROR', payload: error });
  };
  
  return {
    error: state.error,
    setError,
  };
};

export const useStatus = () => {
  const { state, dispatch } = useAppState();
  
  const setStatus = (status: string) => {
    dispatch({ type: 'SET_STATUS', payload: status });
  };
  
  return {
    status: state.status,
    setStatus,
  };
};
