/**
 * Message display component for conversation
 */

import React from 'react';
import { Box, Text } from 'ink';
import { ConversationMessage } from '../types/config.js';
import { useTheme } from '../contexts/ThemeContext.js';

interface MessageDisplayProps {
  message: ConversationMessage;
  isLast?: boolean;
}

export const MessageDisplay: React.FC<MessageDisplayProps> = ({ message, isLast = false }) => {
  const theme = useTheme();
  
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

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString();
  };

  return (
    <Box flexDirection="column" marginBottom={isLast ? 0 : 1}>
      {/* Message header */}
      <Box marginBottom={1}>
        <Text color={getRoleColor(message.role)} bold>
          {getRoleIcon(message.role)} {message.role.charAt(0).toUpperCase() + message.role.slice(1)}
        </Text>
        <Text color={theme.colors.muted} dimColor>
          {' '}• {formatTimestamp(message.timestamp)}
        </Text>
      </Box>
      
      {/* Message content */}
      <Box paddingLeft={2}>
        <Text color={theme.colors.text}>
          {message.content}
        </Text>
      </Box>
    </Box>
  );
};
