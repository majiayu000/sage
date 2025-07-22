/**
 * Streaming chat response with real-time tool execution display
 */

import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { ConversationMessage, ToolCall } from '../types/config.js';
import { useTheme } from '../contexts/ThemeContext.js';
import { ToolExecutionPanel } from './ToolExecutionPanel.js';
import { LoadingIndicator } from './LoadingIndicator.js';

interface StreamingChatResponseProps {
  message: ConversationMessage;
  isStreaming?: boolean;
  toolCalls?: ToolCall[];
  onToolUpdate?: (toolCall: ToolCall) => void;
}

export const StreamingChatResponse: React.FC<StreamingChatResponseProps> = ({ 
  message, 
  isStreaming = false,
  toolCalls = [],
  onToolUpdate 
}) => {
  const theme = useTheme();
  const [displayedContent, setDisplayedContent] = useState('');
  const [currentIndex, setCurrentIndex] = useState(0);

  // Simulate streaming text effect
  useEffect(() => {
    if (!isStreaming) {
      setDisplayedContent(message.content);
      setCurrentIndex(message.content.length);
      return;
    }

    if (currentIndex < message.content.length) {
      const timer = setTimeout(() => {
        setDisplayedContent(message.content.slice(0, currentIndex + 1));
        setCurrentIndex(currentIndex + 1);
      }, 20); // Adjust speed as needed

      return () => clearTimeout(timer);
    }

    // No cleanup needed if currentIndex >= message.content.length
    return undefined;
  }, [message.content, currentIndex, isStreaming]);

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString();
  };

  const getRoleIcon = (role: string) => {
    switch (role) {
      case 'user':
        return '👤';
      case 'assistant':
        return '🤖';
      case 'system':
        return '⚙️';
      default:
        return '💬';
    }
  };

  const getRoleColor = (role: string) => {
    switch (role) {
      case 'user':
        return theme.colors.accent;
      case 'assistant':
        return theme.colors.primary;
      case 'system':
        return theme.colors.muted;
      default:
        return theme.colors.text;
    }
  };

  return (
    <Box flexDirection="column" marginBottom={1}>
      {/* Message header */}
      <Box marginBottom={1}>
        <Text color={getRoleColor(message.role)} bold>
          {getRoleIcon(message.role)} {message.role.charAt(0).toUpperCase() + message.role.slice(1)}
        </Text>
        <Text color={theme.colors.muted} dimColor>
          {' '}• {formatTimestamp(message.timestamp)}
        </Text>
        {isStreaming && (
          <Text color={theme.colors.info}>
            {' '}• Streaming...
          </Text>
        )}
      </Box>

      {/* Tool execution panel */}
      {toolCalls.length > 0 && (
        <ToolExecutionPanel 
          toolCalls={toolCalls} 
          onToolUpdate={onToolUpdate}
        />
      )}

      {/* Message content */}
      <Box paddingLeft={2} flexDirection="column">
        <Text color={theme.colors.text}>
          {displayedContent}
        </Text>
        
        {/* Streaming cursor */}
        {isStreaming && currentIndex < message.content.length && (
          <Text color={theme.colors.accent}>▋</Text>
        )}
        
        {/* Loading indicator for tool execution */}
        {isStreaming && toolCalls.some(tool => tool.status === 'running') && (
          <Box marginTop={1}>
            <LoadingIndicator 
              message="Executing tools..." 
              showTimer={true}
            />
          </Box>
        )}
      </Box>

      {/* Tool results summary */}
      {!isStreaming && toolCalls.length > 0 && (
        <Box paddingLeft={2} marginTop={1}>
          <ToolResultsSummary toolCalls={toolCalls} />
        </Box>
      )}
    </Box>
  );
};

// Tool results summary component
const ToolResultsSummary: React.FC<{ toolCalls: ToolCall[] }> = ({ toolCalls }) => {
  const theme = useTheme();
  
  const completedTools = toolCalls.filter(tool => tool.status === 'completed');
  const failedTools = toolCalls.filter(tool => tool.status === 'failed');
  
  if (completedTools.length === 0 && failedTools.length === 0) {
    return null;
  }

  return (
    <Box flexDirection="column">
      <Text color={theme.colors.muted} dimColor>
        Tool execution summary:
      </Text>
      
      {completedTools.length > 0 && (
        <Text color={theme.colors.success}>
          ✅ {completedTools.length} tool{completedTools.length > 1 ? 's' : ''} completed successfully
        </Text>
      )}
      
      {failedTools.length > 0 && (
        <Text color={theme.colors.error}>
          ❌ {failedTools.length} tool{failedTools.length > 1 ? 's' : ''} failed
        </Text>
      )}
      
      {/* Show concurrent execution stats */}
      {toolCalls.length > 1 && (
        <Text color={theme.colors.info}>
          ⚡ Executed {toolCalls.length} tools with intelligent concurrency
        </Text>
      )}
    </Box>
  );
};
