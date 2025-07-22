/**
 * Interactive mode component
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import TextInput from 'ink-text-input';
import { SageConfig, CliArgs, ConversationMessage, ToolCall as ConfigToolCall } from '../types/config.js';
import { useTheme } from '../contexts/ThemeContext.js';
import { LoadingIndicator } from './LoadingIndicator.js';
import { MessageDisplay } from './MessageDisplay.js';
import { StreamingChatResponse } from './StreamingChatResponse.js';

import { createRpcClient, ChatRequest } from '../utils/rpc-client.js';
import { createDenoClient, isDenoEnvironment, ToolCallStatus } from '../utils/deno-client.js';

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
  const [client] = useState(() => {
    // For now, use a direct backend client instead of RPC
    return createDirectBackendClient();
  });
  const theme = useTheme();

  // Convert ToolCallStatus to ToolCall
  const convertToolCallStatus = (status: ToolCallStatus): ConfigToolCall => ({
    id: status.id,
    name: status.name,
    args: status.args,
    status: status.status,
    startTime: status.start_time,
    endTime: status.end_time,
    result: status.result,
    error: status.error,
  });



  // Add welcome message
  useEffect(() => {
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
  }, []);

  // Handle input submission
  const handleSubmit = async (input: string) => {
    if (!input.trim()) return;

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

    try {
      // Handle special commands
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

      if (input.trim().toLowerCase() === 'config') {
        try {
          const configInfo = await client.getConfig(args.configFile);
          const configMessage: ConversationMessage = {
            role: 'assistant',
            content: `Current Configuration:
• Provider: ${configInfo.provider}
• Model: ${configInfo.model}
• Max Steps: ${configInfo.max_steps}
• Working Directory: ${configInfo.working_directory}
• Verbose: ${configInfo.verbose ? 'Enabled' : 'Disabled'}`,
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

      // Call the actual Sage Agent backend
      const chatRequest: ChatRequest = {
        message: input.trim(),
        config_file: args.configFile,
        working_dir: args.workingDir,
      };

      const response = await client.chat(chatRequest);

      if (response.success) {
        const responseMessage: ConversationMessage = {
          role: (response.role as 'user' | 'assistant' | 'system') || 'assistant',
          content: response.content || '',
          timestamp: response.timestamp || new Date().toISOString(),
          metadata: {
            toolCalls: response.tool_calls?.map(convertToolCallStatus) || [],
          },
        };

        // Set tool calls for real-time display
        if (response.tool_calls && response.tool_calls.length > 0) {
          setCurrentToolCalls(response.tool_calls.map(convertToolCallStatus));
          setStreamingMessageId(responseMessage.timestamp); // Use timestamp as ID
        }

        setMessages(prev => [...prev, responseMessage]);
      } else {
        const errorMessage: ConversationMessage = {
          role: 'assistant',
          content: `Sorry, I encountered an error: ${response.error || 'Unknown error'}`,
          timestamp: new Date().toISOString(),
        };

        setMessages(prev => [...prev, errorMessage]);
      }
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
      process.exit(0);
    }

    if (key.ctrl && input === 'l') {
      setMessages([]);
    }
  });

  return (
    <Box flexDirection="column" height="100%">
      {/* Messages area */}
      <Box flexDirection="column" flexGrow={1} paddingX={1}>
        {messages.map((message, index) => {
          const isStreaming = streamingMessageId === message.timestamp && isProcessing;
          const toolCalls = message.metadata?.toolCalls || [];

          // Use StreamingChatResponse for messages with tool calls or streaming
          if (toolCalls.length > 0 || isStreaming) {
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
