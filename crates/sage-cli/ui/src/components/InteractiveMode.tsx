/**
 * Interactive mode component with IPC-based backend communication
 */

import React, { useState, useEffect, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import TextInput from 'ink-text-input';
import { SageConfig, CliArgs, ConversationMessage, ToolCall as ConfigToolCall } from '../types/config.js';
import { useTheme } from '../contexts/ThemeContext.js';
import { LoadingIndicator } from './LoadingIndicator.js';
import { MessageDisplay } from './MessageDisplay.js';
import { StreamingChatResponse } from './StreamingChatResponse.js';
import {
  IpcClient,
  createIpcClient,
  ToolStartedEvent,
  ToolCompletedEvent,
  LlmThinkingEvent,
  ChatCompletedEvent,
  ErrorEvent,
} from '../utils/ipc-client.js';

interface InteractiveModeProps {
  config: SageConfig;
  args: CliArgs;
}

export const InteractiveMode: React.FC<InteractiveModeProps> = ({ config, args }) => {
  const [messages, setMessages] = useState<ConversationMessage[]>([]);
  const [currentInput, setCurrentInput] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [showInput, setShowInput] = useState(true);
  const [currentToolCalls, setCurrentToolCalls] = useState<ConfigToolCall[]>([]);
  const [streamingMessageId, setStreamingMessageId] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(true);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const clientRef = useRef<IpcClient | null>(null);
  const theme = useTheme();
  const { exit } = useApp();

  // Initialize IPC client
  useEffect(() => {
    let mounted = true;

    const initClient = async () => {
      try {
        const client = await createIpcClient(args.configFile);

        if (!mounted) {
          await client.shutdown();
          return;
        }

        clientRef.current = client;

        // Set up event listeners for real-time updates
        client.on('tool_started', (event: ToolStartedEvent) => {
          setCurrentToolCalls(prev => [
            ...prev,
            {
              id: event.tool_id,
              name: event.tool_name,
              args: event.args,
              status: 'running',
            },
          ]);
        });

        client.on('tool_completed', (event: ToolCompletedEvent) => {
          setCurrentToolCalls(prev =>
            prev.map(tc =>
              tc.id === event.tool_id
                ? {
                    ...tc,
                    status: event.success ? 'completed' : 'failed',
                    result: event.output,
                    error: event.error,
                    endTime: Date.now(),
                  }
                : tc
            )
          );
        });

        client.on('llm_thinking', (_event: LlmThinkingEvent) => {
          // Could show a thinking indicator here
        });

        client.on('error', (event: ErrorEvent) => {
          console.error('IPC Error:', event.message);
        });

        client.on('exit', (code: number) => {
          if (mounted) {
            setConnectionError(`Backend exited with code ${code}`);
          }
        });

        setIsConnecting(false);

        // Add welcome message
        const welcomeMessage: ConversationMessage = {
          role: 'assistant',
          content: `Welcome to Sage Agent Interactive Mode!

I'm your AI assistant for software engineering tasks. I can help you with:
• Code analysis and refactoring
• Bug fixing and debugging
• Documentation generation
• Project setup and configuration
• And much more!

Type your request below, or type 'help' for more information.`,
          timestamp: new Date().toISOString(),
        };

        setMessages([welcomeMessage]);
      } catch (error) {
        if (mounted) {
          setConnectionError(
            error instanceof Error ? error.message : 'Failed to connect to backend'
          );
          setIsConnecting(false);
        }
      }
    };

    initClient();

    return () => {
      mounted = false;
      if (clientRef.current) {
        clientRef.current.shutdown().catch(() => {});
      }
    };
  }, [args.configFile]);

  // Handle input submission
  const handleSubmit = async (input: string) => {
    if (!input.trim() || !clientRef.current) return;

    // Add user message
    const userMessage: ConversationMessage = {
      role: 'user',
      content: input.trim(),
      timestamp: new Date().toISOString(),
    };

    setMessages(prev => [...prev, userMessage]);
    setCurrentInput('');
    setIsProcessing(true);
    setShowInput(false);
    setCurrentToolCalls([]);

    try {
      // Handle special commands locally
      if (input.trim().toLowerCase() === 'help') {
        const helpMessage: ConversationMessage = {
          role: 'assistant',
          content: `Available commands:
• help - Show this help message
• clear - Clear conversation history
• config - Show current configuration
• exit - Exit interactive mode

You can also ask me to perform any software engineering task!`,
          timestamp: new Date().toISOString(),
        };

        setMessages(prev => [...prev, helpMessage]);
        setIsProcessing(false);
        setShowInput(true);
        return;
      }

      if (input.trim().toLowerCase() === 'clear') {
        setMessages([]);
        setIsProcessing(false);
        setShowInput(true);
        return;
      }

      if (input.trim().toLowerCase() === 'exit') {
        await clientRef.current.shutdown();
        exit();
        return;
      }

      if (input.trim().toLowerCase() === 'config') {
        try {
          const configInfo = await clientRef.current.getConfig(args.configFile);
          const configMessage: ConversationMessage = {
            role: 'assistant',
            content: `Current Configuration:
• Provider: ${configInfo.provider}
• Model: ${configInfo.model}
• Max Steps: ${configInfo.max_steps ?? 'unlimited'}
• Working Directory: ${configInfo.working_directory}
• Token Budget: ${configInfo.total_token_budget ?? 'unlimited'}`,
            timestamp: new Date().toISOString(),
          };

          setMessages(prev => [...prev, configMessage]);
        } catch (error) {
          const errorMessage: ConversationMessage = {
            role: 'assistant',
            content: `Failed to load configuration: ${error instanceof Error ? error.message : 'Unknown error'}`,
            timestamp: new Date().toISOString(),
          };

          setMessages(prev => [...prev, errorMessage]);
        }
        setIsProcessing(false);
        setShowInput(true);
        return;
      }

      // Send chat request to backend
      const messageId = Date.now().toString();
      setStreamingMessageId(messageId);

      const response = await clientRef.current.chat({
        message: input.trim(),
        config_file: args.configFile,
        working_dir: args.workingDir,
      });

      // Create response message with tool results
      const responseMessage: ConversationMessage = {
        role: 'assistant',
        content: response.content || '',
        timestamp: new Date().toISOString(),
        metadata: {
          toolCalls: response.tool_results?.map((tr) => ({
            id: tr.tool_id,
            name: tr.tool_name,
            args: {},
            status: tr.success ? 'completed' as const : 'failed' as const,
            result: tr.output,
            error: tr.error,
            endTime: Date.now(),
          })) || [],
        },
      };

      setMessages(prev => [...prev, responseMessage]);
    } catch (error) {
      const errorMessage: ConversationMessage = {
        role: 'assistant',
        content: `Sorry, I encountered an error: ${error instanceof Error ? error.message : 'Unknown error'}`,
        timestamp: new Date().toISOString(),
      };

      setMessages(prev => [...prev, errorMessage]);
    } finally {
      setIsProcessing(false);
      setShowInput(true);
      setStreamingMessageId(null);
    }
  };

  // Handle keyboard shortcuts
  useInput((input, key) => {
    if (key.ctrl && input === 'c') {
      // If there are messages, copy the last assistant response to stdout
      if (messages.length > 0) {
        const lastAssistantMessage = messages
          .slice()
          .reverse()
          .find(msg => msg.role === 'assistant');

        if (lastAssistantMessage) {
          // Write to stdout so it can be selected/copied
          process.stdout.write('\n--- Last Response (Copyable) ---\n');
          process.stdout.write(lastAssistantMessage.content);
          process.stdout.write('\n--- End Response ---\n\n');
          return;
        }
      }
      // Clean shutdown
      if (clientRef.current) {
        clientRef.current.shutdown().catch(() => {});
      }
      exit();
    }

    if (key.ctrl && input === 'l') {
      setMessages([]);
    }
  });

  // Show connecting state
  if (isConnecting) {
    return (
      <Box flexDirection="column" height="100%" justifyContent="center" alignItems="center">
        <LoadingIndicator message="Connecting to Sage backend..." showTimer={true} />
      </Box>
    );
  }

  // Show connection error
  if (connectionError) {
    return (
      <Box flexDirection="column" height="100%" padding={1}>
        <Text color="red" bold>Connection Error</Text>
        <Text color="red">{connectionError}</Text>
        <Text> </Text>
        <Text>Please ensure the Sage backend is properly installed.</Text>
        <Text>Try running: cargo build -p sage-cli</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" height="100%">
      {/* Messages area */}
      <Box flexDirection="column" flexGrow={1} paddingX={1}>
        {messages.map((message, index) => {
          const isStreaming = streamingMessageId !== null && index === messages.length - 1 && isProcessing;
          const toolCalls = message.metadata?.toolCalls || [];

          // Use StreamingChatResponse for messages with tool calls or streaming
          if (toolCalls.length > 0 || (isStreaming && currentToolCalls.length > 0)) {
            return (
              <StreamingChatResponse
                key={index}
                message={message}
                isStreaming={isStreaming}
                toolCalls={isStreaming ? currentToolCalls : toolCalls}
              />
            );
          }

          // Use regular MessageDisplay for simple messages
          return (
            <MessageDisplay
              key={index}
              message={message}
              isLast={index === messages.length - 1}
            />
          );
        })}

        {isProcessing && (
          <Box marginY={1}>
            <LoadingIndicator
              message="Processing your request..."
              showTimer={true}
            />
          </Box>
        )}
      </Box>

      {/* Input area */}
      {showInput && (
        <Box
          borderStyle="single"
          borderColor={theme.colors.border}
          paddingX={1}
          marginX={1}
          marginBottom={1}
        >
          <Box alignItems="center" width="100%">
            <Text color={theme.colors.accent} bold>
              {'> '}
            </Text>
            <TextInput
              value={currentInput}
              onChange={setCurrentInput}
              onSubmit={handleSubmit}
              placeholder="Type your request here..."
            />
          </Box>
        </Box>
      )}
    </Box>
  );
};
