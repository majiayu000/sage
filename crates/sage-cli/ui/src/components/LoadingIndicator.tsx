/**
 * Loading indicator component with timer and animations
 */

import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { useTheme } from '../contexts/ThemeContext.js';

interface LoadingIndicatorProps {
  message?: string;
  showTimer?: boolean;
  spinnerType?: 'dots' | 'line' | 'pipe' | 'star' | 'arrow';
}

export const LoadingIndicator: React.FC<LoadingIndicatorProps> = ({ 
  message = 'Loading...', 
  showTimer = false,
  spinnerType = 'dots'
}) => {
  const [elapsedTime, setElapsedTime] = useState(0);
  const theme = useTheme();

  useEffect(() => {
    if (!showTimer) return;

    const startTime = Date.now();
    const interval = setInterval(() => {
      setElapsedTime(Date.now() - startTime);
    }, 100);

    return () => clearInterval(interval);
  }, [showTimer]);

  const formatTime = (ms: number): string => {
    const seconds = Math.floor(ms / 1000);
    const milliseconds = Math.floor((ms % 1000) / 100);
    return `${seconds}.${milliseconds}s`;
  };

  return (
    <Box alignItems="center">
      <Box marginRight={1}>
        <Spinner type={spinnerType} />
      </Box>
      
      <Text color={theme.colors.primary}>
        {message}
      </Text>
      
      {showTimer && (
        <Text color={theme.colors.muted} dimColor>
          {' '}({formatTime(elapsedTime)})
        </Text>
      )}
    </Box>
  );
};
