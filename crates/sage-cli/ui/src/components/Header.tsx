/**
 * Header component for Sage Agent CLI
 */

import React from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/ThemeContext.js';

interface HeaderProps {
  title: string;
  subtitle?: string;
  showVersion?: boolean;
}

export const Header: React.FC<HeaderProps> = ({ 
  title, 
  subtitle, 
  showVersion = true 
}) => {
  const theme = useTheme();
  
  return (
    <Box 
      borderStyle="round" 
      borderColor={theme.colors.border}
      paddingX={1}
      marginBottom={1}
    >
      <Box flexDirection="column" width="100%">
        <Box justifyContent="space-between" alignItems="center">
          <Box>
            <Text bold color={theme.colors.primary}>
              {title}
            </Text>
            {subtitle && (
              <Text color={theme.colors.secondary} dimColor>
                {' '}- {subtitle}
              </Text>
            )}
          </Box>
          
          {showVersion && (
            <Text color={theme.colors.muted} dimColor>
              v0.1.0
            </Text>
          )}
        </Box>
      </Box>
    </Box>
  );
};
