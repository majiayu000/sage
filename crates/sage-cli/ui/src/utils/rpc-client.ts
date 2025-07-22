/**
 * JSON-RPC client for communicating with Rust backend
 */

import axios, { AxiosInstance } from 'axios';

export interface ChatMessage {
  role: string;
  content: string;
  timestamp: string;
}

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

export class SageRpcClient {
  private client: AxiosInstance;
  private requestId: number = 1;

  constructor(baseUrl: string = 'http://127.0.0.1:3030') {
    this.client = axios.create({
      baseURL: baseUrl,
      headers: {
        'Content-Type': 'application/json',
      },
      timeout: 30000, // 30 seconds timeout
    });
  }

  private async makeRpcCall<T>(method: string, params: any): Promise<T> {
    const request = {
      jsonrpc: '2.0',
      method,
      params,
      id: this.requestId++,
    };

    try {
      const response = await this.client.post('/', request);
      
      if (response.data.error) {
        throw new Error(`RPC Error: ${response.data.error.message}`);
      }
      
      return response.data.result;
    } catch (error) {
      if (axios.isAxiosError(error)) {
        if (error.code === 'ECONNREFUSED') {
          throw new Error('Cannot connect to Sage backend. Please ensure the RPC server is running.');
        }
        throw new Error(`Network error: ${error.message}`);
      }
      throw error;
    }
  }

  /**
   * Send a chat message and get AI response
   */
  async chat(request: ChatRequest): Promise<ChatResponse> {
    return this.makeRpcCall<ChatResponse>('chat', request);
  }

  /**
   * Get current configuration
   */
  async getConfig(configFile: string): Promise<ConfigInfo> {
    return this.makeRpcCall<ConfigInfo>('get_config', configFile);
  }

  /**
   * Validate configuration
   */
  async validateConfig(configFile: string): Promise<boolean> {
    return this.makeRpcCall<boolean>('validate_config', configFile);
  }

  /**
   * List available tools
   */
  async listTools(): Promise<string[]> {
    return this.makeRpcCall<string[]>('list_tools', null);
  }

  /**
   * Test connection to the RPC server
   */
  async testConnection(): Promise<boolean> {
    try {
      await this.listTools();
      return true;
    } catch (error) {
      return false;
    }
  }
}

/**
 * Get RPC client instance
 */
export function createRpcClient(): SageRpcClient {
  const port = process.env.SAGE_RPC_PORT || '3030';
  return new SageRpcClient(`http://127.0.0.1:${port}`);
}
