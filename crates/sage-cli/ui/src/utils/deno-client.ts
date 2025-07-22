/**
 * Deno ops client for communicating with embedded Rust backend
 */

export interface ChatRequest {
  message: string;
  config_file: string;
  working_dir?: string;
}

export interface ChatResponse {
  role: string;
  content: string;
  timestamp: string;
  success: boolean;
  error?: string;
  tool_calls: ToolCallStatus[];
}

export interface ToolCallStatus {
  id: string;
  name: string;
  args: any;
  status: 'queued' | 'running' | 'completed' | 'failed';
  start_time?: number;
  end_time?: number;
  result?: string;
  error?: string;
}

export interface ConfigInfo {
  provider: string;
  model: string;
  max_steps: number;
  working_directory: string;
  verbose: boolean;
}

// Declare Deno ops (these will be available in the embedded runtime)
declare global {
  const Deno: {
    core: {
      ops: {
        op_sage_chat(request: ChatRequest): Promise<ChatResponse>;
        op_sage_get_config(configFile: string): Promise<ConfigInfo>;
        op_sage_list_tools(): Promise<string[]>;
      };
    };
  };
}

export class SageDenoClient {
  /**
   * Send a chat message and get AI response
   */
  async chat(request: ChatRequest): Promise<ChatResponse> {
    try {
      // Call the Rust backend through the global interface
      const backend = (globalThis as any).__SAGE_BACKEND__;
      if (!backend) {
        throw new Error('Sage backend not available');
      }

      return await backend.chat(request);
    } catch (error) {
      return {
        role: 'assistant',
        content: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`,
        timestamp: new Date().toISOString(),
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
        tool_calls: [],
      };
    }
  }

  /**
   * Get current configuration
   */
  async getConfig(configFile: string): Promise<ConfigInfo> {
    const backend = (globalThis as any).__SAGE_BACKEND__;
    if (!backend) {
      throw new Error('Sage backend not available');
    }
    return await backend.getConfig(configFile);
  }

  /**
   * List available tools
   */
  async listTools(): Promise<string[]> {
    const backend = (globalThis as any).__SAGE_BACKEND__;
    if (!backend) {
      throw new Error('Sage backend not available');
    }
    return await backend.listTools();
  }

  /**
   * Test connection (always returns true for embedded runtime)
   */
  async testConnection(): Promise<boolean> {
    try {
      await this.listTools();
      return true;
    } catch {
      return false;
    }
  }
}

/**
 * Get client instance
 */
export function createDenoClient(): SageDenoClient {
  return new SageDenoClient();
}

/**
 * Check if running in Deno embedded environment
 */
export function isDenoEnvironment(): boolean {
  // Check if we're running in a Sage Agent context with Rust backend
  return typeof (globalThis as any).__SAGE_BACKEND__ !== 'undefined';
}
