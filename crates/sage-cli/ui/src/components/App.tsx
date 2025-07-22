/**
 * Main App component for Sage Agent CLI
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { AppProps } from '../types/config.js';
import { Header } from './Header.js';
import { Footer } from './Footer.js';
import { InteractiveMode } from './InteractiveMode.js';
import { CommandMode } from './CommandMode.js';
import { LoadingIndicator } from './LoadingIndicator.js';
import { ErrorDisplay } from './ErrorDisplay.js';
import { ThemeProvider } from '../contexts/ThemeContext.js';
import { AppStateProvider } from '../contexts/AppStateContext.js';

export const App: React.FC<AppProps> = ({ config, args, mode }) => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<'main' | 'help' | 'settings'>('main');
  const { exit } = useApp();

  // Initialize app
  useEffect(() => {
    const initializeApp = async () => {
      try {
        // Simulate initialization delay
        await new Promise(resolve => setTimeout(resolve, 1000));
        setIsLoading(false);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error occurred');
        setIsLoading(false);
      }
    };

    initializeApp();
  }, []);

  // Handle global keyboard shortcuts
  useInput((input, key) => {
    if (key.ctrl && input === 'c') {
      exit();
      return;
    }

    if (key.ctrl && input === 'h') {
      setCurrentView(currentView === 'help' ? 'main' : 'help');
      return;
    }

    if (key.ctrl && input === 's') {
      setCurrentView(currentView === 'settings' ? 'main' : 'settings');
      return;
    }

    if (key.escape) {
      setCurrentView('main');
      return;
    }
  });

  if (error) {
    return (
      <ThemeProvider theme={config.ui?.theme || 'default'}>
        <Box flexDirection="column" height="100%">
          <Header title="Sage Agent" subtitle="Error" />
          <Box flexGrow={1}>
            <ErrorDisplay error={error} />
          </Box>
          <Footer 
            shortcuts={[
              { key: 'Ctrl+C', description: 'Exit' }
            ]}
          />
        </Box>
      </ThemeProvider>
    );
  }

  if (isLoading) {
    return (
      <ThemeProvider theme={config.ui?.theme || 'default'}>
        <Box flexDirection="column" height="100%" justifyContent="center" alignItems="center">
          <LoadingIndicator 
            message="Initializing Sage Agent..." 
            showTimer={true}
          />
        </Box>
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider theme={config.ui?.theme || 'default'}>
      <AppStateProvider config={config} args={args}>
        <Box flexDirection="column" height="100%">
          <Header 
            title="Sage Agent" 
            subtitle={mode === 'interactive' ? 'Interactive Mode' : 'Command Mode'}
          />
          
          <Box flexGrow={1}>
            {currentView === 'help' && (
              <HelpView />
            )}
            {currentView === 'settings' && (
              <SettingsView config={config} />
            )}
            {currentView === 'main' && (
              mode === 'interactive' ? (
                <InteractiveMode config={config} args={args} />
              ) : (
                <CommandMode config={config} args={args} />
              )
            )}
          </Box>

          <Footer
            shortcuts={[
              { key: 'Ctrl+C', description: 'Copy Last Response/Exit' },
              { key: 'Ctrl+L', description: 'Clear' },
              { key: 'Ctrl+H', description: 'Help' },
              { key: 'Esc', description: 'Back' }
            ]}
          />
        </Box>
      </AppStateProvider>
    </ThemeProvider>
  );
};

// Help view component
const HelpView: React.FC = () => {
  return (
    <Box flexDirection="column" padding={1}>
      <Text bold>Sage Agent - Help</Text>
      <Text> </Text>
      <Text>Commands:</Text>
      <Text>  sage run &lt;task&gt;        Run a specific task</Text>
      <Text>  sage interactive        Start interactive mode</Text>
      <Text>  sage config &lt;action&gt;    Manage configuration</Text>
      <Text>  sage trajectory &lt;action&gt; Manage trajectories</Text>
      <Text>  sage tools              Show available tools</Text>
      <Text> </Text>
      <Text>Keyboard Shortcuts:</Text>
      <Text>  Ctrl+C                  Copy last response to stdout (or exit if no messages)</Text>
      <Text>  Ctrl+L                  Clear conversation history</Text>
      <Text>  Ctrl+H                  Toggle help</Text>
      <Text>  Esc                     Go back</Text>
      <Text> </Text>
      <Text>For more information, visit: https://github.com/your-repo/sage-agent</Text>
    </Box>
  );
};

// Settings view component
const SettingsView: React.FC<{ config: any }> = ({ config }) => {
  return (
    <Box flexDirection="column" padding={1}>
      <Text bold>Sage Agent - Settings</Text>
      <Text> </Text>
      <Text>Current Configuration:</Text>
      <Text>  Provider: {config.llm?.provider || 'Not set'}</Text>
      <Text>  Model: {config.llm?.model || 'Not set'}</Text>
      <Text>  Max Steps: {config.max_steps || 'Not set'}</Text>
      <Text>  Working Directory: {config.working_directory || process.cwd()}</Text>
      <Text>  Theme: {config.ui?.theme || 'default'}</Text>
      <Text>  Animations: {config.ui?.animations ? 'Enabled' : 'Disabled'}</Text>
      <Text> </Text>
      <Text>To modify settings, edit your configuration file or use environment variables.</Text>
    </Box>
  );
};
