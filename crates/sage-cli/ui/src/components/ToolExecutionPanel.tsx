/**
 * Tool execution panel showing concurrent tool calls with real-time status
 */

import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { useTheme } from '../contexts/ThemeContext.js';

import { ToolCall } from '../types/config.js';

interface ToolExecutionPanelProps {
  toolCalls: ToolCall[];
  onToolUpdate?: (toolCall: ToolCall) => void;
}

export const ToolExecutionPanel: React.FC<ToolExecutionPanelProps> = ({ 
  toolCalls, 
  onToolUpdate 
}) => {
  const theme = useTheme();
  const [currentTime, setCurrentTime] = useState(Date.now());

  // Update current time for duration calculation
  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Date.now());
    }, 100);

    return () => clearInterval(interval);
  }, []);

  if (toolCalls.length === 0) {
    return null;
  }

  const getStatusIcon = (status: ToolCall['status']) => {
    switch (status) {
      case 'queued':
        return <Text>⏳</Text>;
      case 'running':
        return <Spinner type="dots" />;
      case 'completed':
        return <Text>✅</Text>;
      case 'failed':
        return <Text>❌</Text>;
      default:
        return <Text>❓</Text>;
    }
  };

  const getStatusColor = (status: ToolCall['status']) => {
    switch (status) {
      case 'queued':
        return theme.colors.muted;
      case 'running':
        return theme.colors.info;
      case 'completed':
        return theme.colors.success;
      case 'failed':
        return theme.colors.error;
      default:
        return theme.colors.text;
    }
  };

  const formatDuration = (startTime?: number, endTime?: number) => {
    if (!startTime) return '';
    const end = endTime || currentTime;
    const duration = end - startTime;
    return `${(duration / 1000).toFixed(1)}s`;
  };

  const runningTools = toolCalls.filter(tool => tool.status === 'running');
  const queuedTools = toolCalls.filter(tool => tool.status === 'queued');
  const completedTools = toolCalls.filter(tool => tool.status === 'completed');
  const failedTools = toolCalls.filter(tool => tool.status === 'failed');

  return (
    <Box 
      flexDirection="column" 
      borderStyle="round" 
      borderColor={theme.colors.border}
      padding={1}
      marginY={1}
    >
      <Box marginBottom={1}>
        <Text bold color={theme.colors.primary}>
          🔧 Tool Execution Status
        </Text>
        <Text color={theme.colors.muted}>
          {' '}({runningTools.length} running, {queuedTools.length} queued, {completedTools.length} completed, {failedTools.length} failed)
        </Text>
      </Box>

      {/* Running tools */}
      {runningTools.length > 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text bold color={theme.colors.info}>Running:</Text>
          {runningTools.map(tool => (
            <Box key={tool.id} marginLeft={2}>
              <Box marginRight={1}>
                {getStatusIcon(tool.status)}
              </Box>
              <Text color={getStatusColor(tool.status)}>
                {tool.name}
              </Text>
              <Text color={theme.colors.muted}>
                {' '}({formatDuration(tool.startTime)})
              </Text>
            </Box>
          ))}
        </Box>
      )}

      {/* Queued tools */}
      {queuedTools.length > 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text bold color={theme.colors.muted}>Queued:</Text>
          {queuedTools.map(tool => (
            <Box key={tool.id} marginLeft={2}>
              <Box marginRight={1}>
                {getStatusIcon(tool.status)}
              </Box>
              <Text color={getStatusColor(tool.status)}>
                {tool.name}
              </Text>
            </Box>
          ))}
        </Box>
      )}

      {/* Completed tools */}
      {completedTools.length > 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text bold color={theme.colors.success}>Completed:</Text>
          {completedTools.map(tool => (
            <Box key={tool.id} marginLeft={2}>
              <Box marginRight={1}>
                {getStatusIcon(tool.status)}
              </Box>
              <Text color={getStatusColor(tool.status)}>
                {tool.name}
              </Text>
              <Text color={theme.colors.muted}>
                {' '}({formatDuration(tool.startTime, tool.endTime)})
              </Text>
            </Box>
          ))}
        </Box>
      )}

      {/* Failed tools */}
      {failedTools.length > 0 && (
        <Box flexDirection="column">
          <Text bold color={theme.colors.error}>Failed:</Text>
          {failedTools.map(tool => (
            <Box key={tool.id} flexDirection="column" marginLeft={2}>
              <Box>
                <Box marginRight={1}>
                  {getStatusIcon(tool.status)}
                </Box>
                <Text color={getStatusColor(tool.status)}>
                  {tool.name}
                </Text>
                <Text color={theme.colors.muted}>
                  {' '}({formatDuration(tool.startTime, tool.endTime)})
                </Text>
              </Box>
              {tool.error && (
                <Text color={theme.colors.error} dimColor>
                  Error: {tool.error}
                </Text>
              )}
            </Box>
          ))}
        </Box>
      )}

      {/* Concurrent execution indicator */}
      {runningTools.length > 1 && (
        <Box marginTop={1}>
          <Text color={theme.colors.accent} bold>
            ⚡ {runningTools.length} tools running concurrently
          </Text>
        </Box>
      )}
    </Box>
  );
};
