/**
 * Configuration types for Sage Agent
 */

export interface LLMConfig {
  provider: 'openai' | 'anthropic' | 'google' | 'ollama';
  model: string;
  api_key?: string;
  base_url?: string;
  max_tokens?: number;
  temperature?: number;
  timeout?: number;
}

export interface ToolConfig {
  name: string;
  enabled: boolean;
  config?: Record<string, any>;
}

export interface SageConfig {
  llm: LLMConfig;
  tools: ToolConfig[];
  max_steps?: number;
  working_directory?: string;
  trajectory_directory?: string;
  verbose?: boolean;
  ui?: {
    theme?: string;
    animations?: boolean;
    colors?: boolean;
  };
}

export interface CliArgs {
  command?: string;
  task?: string;
  provider?: string;
  model?: string;
  modelBaseUrl?: string;
  apiKey?: string;
  maxSteps?: number;
  workingDir?: string;
  configFile: string;
  trajectoryFile?: string;
  patchPath?: string;
  mustPatch?: boolean;
  verbose?: boolean;
  force?: boolean;
  directory?: string;
}

export interface AppProps {
  config: SageConfig;
  args: CliArgs;
  mode: 'interactive' | 'command';
}

export interface TaskMetadata {
  id: string;
  description: string;
  created_at: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  steps?: number;
  max_steps?: number;
}

export interface TrajectoryEntry {
  timestamp: string;
  type: 'user_input' | 'agent_response' | 'tool_call' | 'error';
  content: string;
  metadata?: Record<string, any>;
}

export interface ConversationMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  metadata?: {
    toolCalls?: ToolCall[];
    [key: string]: any;
  };
}

export interface ToolCall {
  id: string;
  name: string;
  args: Record<string, any>;
  status: 'queued' | 'running' | 'completed' | 'failed';
  startTime?: number;
  endTime?: number;
  result?: string;
  error?: string;
}
