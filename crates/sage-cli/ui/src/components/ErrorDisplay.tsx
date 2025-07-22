/**
 * Error display component
 */

import React from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/ThemeContext.js';

interface ErrorDisplayProps {
  error: string;
  details?: string;
  showRetry?: boolean;
  onRetry?: () => void;
}

export const ErrorDisplay: React.FC<ErrorDisplayProps> = ({ 
  error, 
  details, 
  showRetry = false, 
  onRetry 
}) => {
  const theme = useTheme();

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text color={theme.colors.error} bold>
          ❌ Error
        </Text>
      </Box>
      
      <Box marginBottom={1}>
        <Text color={theme.colors.error}>
          {error}
        </Text>
      </Box>
      
      {details && (
        <Box marginBottom={1}>
          <Text color={theme.colors.muted} dimColor>
            Details: {details}
          </Text>
        </Box>
      )}
      
      {showRetry && onRetry && (
        <Box>
          <Text color={theme.colors.accent}>
            Press R to retry, or Ctrl+C to exit
          </Text>
        </Box>
      )}
    </Box>
  );
};
