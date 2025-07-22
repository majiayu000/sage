/**
 * Footer component for Sage Agent CLI
 */

import React from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/ThemeContext.js';

interface Shortcut {
  key: string;
  description: string;
}

interface FooterProps {
  shortcuts: Shortcut[];
  status?: string;
}

export const Footer: React.FC<FooterProps> = ({ shortcuts, status }) => {
  const theme = useTheme();
  
  return (
    <Box 
      borderStyle="round" 
      borderColor={theme.colors.border}
      paddingX={1}
      marginTop={1}
    >
      <Box justifyContent="space-between" alignItems="center" width="100%">
        <Box>
          {shortcuts.map((shortcut, index) => (
            <Box key={shortcut.key} marginRight={2}>
              <Text color={theme.colors.accent} bold>
                {shortcut.key}
              </Text>
              <Text color={theme.colors.muted}>
                {' '}{shortcut.description}
              </Text>
            </Box>
          ))}
        </Box>
        
        {status && (
          <Text color={theme.colors.secondary}>
            {status}
          </Text>
        )}
      </Box>
    </Box>
  );
};
