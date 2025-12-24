/**
 * IPC Client for communicating with Rust backend via stdio
 *
 * This client spawns the Rust backend as a subprocess and communicates
 * via stdin/stdout using JSON-Lines protocol.
 */

import { spawn, ChildProcess } from 'child_process';
import { EventEmitter } from 'events';
import * as readline from 'readline';
import * as path from 'path';

// Request types
export interface ChatRequest {
  message: string;
  config_file?: string;
  working_dir?: string;
  request_id?: string;
}

// Event types from Rust backend
export interface IpcEvent {
  type: string;
  [key: string]: any;
}

export interface ReadyEvent extends IpcEvent {
  type: 'ready';
  version: string;
}

export interface AckEvent extends IpcEvent {
  type: 'ack';
  request_id: string;
}

export interface ToolStartedEvent extends IpcEvent {
  type: 'tool_started';
  request_id: string;
  tool_id: string;
  tool_name: string;
  args?: any;
}

export interface ToolProgressEvent extends IpcEvent {
  type: 'tool_progress';
  request_id: string;
  tool_id: string;
  output: string;
}

export interface ToolCompletedEvent extends IpcEvent {
  type: 'tool_completed';
  request_id: string;
  tool_id: string;
  success: boolean;
  output?: string;
  error?: string;
  duration_ms: number;
}

export interface LlmThinkingEvent extends IpcEvent {
  type: 'llm_thinking';
  request_id: string;
}

export interface LlmChunkEvent extends IpcEvent {
  type: 'llm_chunk';
  request_id: string;
  content: string;
}

export interface LlmDoneEvent extends IpcEvent {
  type: 'llm_done';
  request_id: string;
  content: string;
  tool_calls: Array<{
    id: string;
    name: string;
    args: Record<string, any>;
  }>;
}

export interface ChatCompletedEvent extends IpcEvent {
  type: 'chat_completed';
  request_id: string;
  content: string;
  completed: boolean;
  tool_results: Array<{
    tool_id: string;
    tool_name: string;
    success: boolean;
    output?: string;
    error?: string;
    duration_ms: number;
  }>;
  duration_ms: number;
}

export interface ErrorEvent extends IpcEvent {
  type: 'error';
  request_id?: string;
  code: string;
  message: string;
}

export interface ConfigEvent extends IpcEvent {
  type: 'config';
  provider: string;
  model: string;
  max_steps?: number;
  working_directory: string;
  total_token_budget?: number;
}

export interface ToolsEvent extends IpcEvent {
  type: 'tools';
  tools: Array<{
    name: string;
    description: string;
    parameters: Array<{
      name: string;
      description: string;
      required: boolean;
      param_type: string;
    }>;
  }>;
}

export type IpcEventType =
  | ReadyEvent
  | AckEvent
  | ToolStartedEvent
  | ToolProgressEvent
  | ToolCompletedEvent
  | LlmThinkingEvent
  | LlmChunkEvent
  | LlmDoneEvent
  | ChatCompletedEvent
  | ErrorEvent
  | ConfigEvent
  | ToolsEvent;

/**
 * IPC Client that communicates with the Rust backend
 */
export class IpcClient extends EventEmitter {
  private process: ChildProcess | null = null;
  private readline: readline.Interface | null = null;
  private ready: boolean = false;
  private readyPromise: Promise<void>;
  private readyResolve: (() => void) | null = null;
  private pendingRequests: Map<string, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }> = new Map();
  private requestCounter: number = 0;
  private binaryPath: string;

  constructor(binaryPath?: string) {
    super();

    // Find the sage binary
    this.binaryPath = binaryPath || this.findBinaryPath();

    this.readyPromise = new Promise((resolve) => {
      this.readyResolve = resolve;
    });
  }

  /**
   * Find the path to the sage binary
   */
  private findBinaryPath(): string {
    // Try common locations
    const candidates = [
      // Development: relative to ui directory
      path.join(__dirname, '../../../../target/debug/sage'),
      path.join(__dirname, '../../../../target/release/sage'),
      // Installed globally
      'sage',
    ];

    // For now, use the first one (development path)
    // In production, we'd check which one exists
    return candidates[0];
  }

  /**
   * Start the backend process
   */
  async start(configFile?: string): Promise<void> {
    if (this.process) {
      throw new Error('IPC client already started');
    }

    const args = ['ipc'];
    if (configFile) {
      args.push('--config-file', configFile);
    }

    this.process = spawn(this.binaryPath, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    // Handle stderr (for debugging)
    this.process.stderr?.on('data', (data) => {
      console.error('[IPC Backend]', data.toString());
    });

    // Handle process exit
    this.process.on('exit', (code) => {
      this.emit('exit', code);
      this.cleanup();
    });

    this.process.on('error', (error) => {
      this.emit('error', error);
      this.cleanup();
    });

    // Set up readline for stdout
    if (this.process.stdout) {
      this.readline = readline.createInterface({
        input: this.process.stdout,
        crlfDelay: Infinity,
      });

      this.readline.on('line', (line) => {
        this.handleLine(line);
      });
    }

    // Wait for ready event
    await this.readyPromise;
  }

  /**
   * Handle a line of output from the backend
   */
  private handleLine(line: string): void {
    if (!line.trim()) return;

    try {
      const event = JSON.parse(line) as IpcEventType;

      // Handle ready event
      if (event.type === 'ready') {
        this.ready = true;
        this.readyResolve?.();
        this.emit('ready', event);
        return;
      }

      // Emit the event
      this.emit('event', event);
      this.emit(event.type, event);

      // Handle request completion
      if ('request_id' in event && event.request_id) {
        const pending = this.pendingRequests.get(event.request_id);

        if (event.type === 'chat_completed') {
          pending?.resolve(event);
          this.pendingRequests.delete(event.request_id);
        } else if (event.type === 'error') {
          pending?.reject(new Error(event.message));
          this.pendingRequests.delete(event.request_id);
        } else if (event.type === 'config') {
          pending?.resolve(event);
          this.pendingRequests.delete(event.request_id);
        } else if (event.type === 'tools') {
          pending?.resolve(event);
          this.pendingRequests.delete(event.request_id);
        }
      }
    } catch (error) {
      console.error('Failed to parse IPC event:', line, error);
    }
  }

  /**
   * Send a request to the backend
   */
  private sendRequest(method: string, params: any = {}): string {
    if (!this.process?.stdin) {
      throw new Error('IPC client not started');
    }

    const requestId = `req-${++this.requestCounter}`;
    const request = {
      method,
      params: { ...params, request_id: requestId },
    };

    const line = JSON.stringify(request) + '\n';
    this.process.stdin.write(line);
    return requestId;
  }

  /**
   * Send a chat message and wait for completion
   */
  async chat(request: ChatRequest): Promise<ChatCompletedEvent> {
    if (!this.ready) {
      await this.readyPromise;
    }

    const requestId = this.sendRequest('chat', request);

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, { resolve, reject });

      // Timeout after 5 minutes
      setTimeout(() => {
        if (this.pendingRequests.has(requestId)) {
          this.pendingRequests.delete(requestId);
          reject(new Error('Request timeout'));
        }
      }, 5 * 60 * 1000);
    });
  }

  /**
   * Get current configuration
   */
  async getConfig(configFile: string): Promise<ConfigEvent> {
    if (!this.ready) {
      await this.readyPromise;
    }

    const requestId = this.sendRequest('get_config', { config_file: configFile });

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, { resolve, reject });

      setTimeout(() => {
        if (this.pendingRequests.has(requestId)) {
          this.pendingRequests.delete(requestId);
          reject(new Error('Request timeout'));
        }
      }, 10000);
    });
  }

  /**
   * List available tools
   */
  async listTools(): Promise<ToolsEvent> {
    if (!this.ready) {
      await this.readyPromise;
    }

    const requestId = this.sendRequest('list_tools', {});

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, { resolve, reject });

      setTimeout(() => {
        if (this.pendingRequests.has(requestId)) {
          this.pendingRequests.delete(requestId);
          reject(new Error('Request timeout'));
        }
      }, 10000);
    });
  }

  /**
   * Ping the backend
   */
  ping(): void {
    this.sendRequest('ping', {});
  }

  /**
   * Shutdown the backend
   */
  async shutdown(): Promise<void> {
    if (!this.process?.stdin) {
      return;
    }

    this.sendRequest('shutdown', {});

    // Wait for process to exit
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => {
        this.process?.kill();
        resolve();
      }, 5000);

      this.process?.on('exit', () => {
        clearTimeout(timeout);
        resolve();
      });
    });

    this.cleanup();
  }

  /**
   * Clean up resources
   */
  private cleanup(): void {
    this.readline?.close();
    this.readline = null;
    this.process = null;
    this.ready = false;

    // Reject all pending requests
    for (const [, { reject }] of this.pendingRequests) {
      reject(new Error('IPC client closed'));
    }
    this.pendingRequests.clear();
  }

  /**
   * Check if the client is ready
   */
  isReady(): boolean {
    return this.ready;
  }
}

/**
 * Create and start an IPC client
 */
export async function createIpcClient(configFile?: string, binaryPath?: string): Promise<IpcClient> {
  const client = new IpcClient(binaryPath);
  await client.start(configFile);
  return client;
}
