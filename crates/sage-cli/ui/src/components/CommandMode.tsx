/**
 * Command mode component for non-interactive commands
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useApp } from 'ink';
import { SageConfig, CliArgs } from '../types/config.js';
import { useTheme } from '../contexts/ThemeContext.js';
import { LoadingIndicator } from './LoadingIndicator.js';
import { ErrorDisplay } from './ErrorDisplay.js';

interface CommandModeProps {
  config: SageConfig;
  args: CliArgs;
}

export const CommandMode: React.FC<CommandModeProps> = ({ config, args }) => {
  const [isProcessing, setIsProcessing] = useState(true);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const theme = useTheme();
  const { exit } = useApp();

  useEffect(() => {
    const executeCommand = async () => {
      try {
        setIsProcessing(true);
        
        // Determine which command to execute
        const command = args.command || (args.task ? 'run' : 'help');
        
        switch (command) {
          case 'run':
            await executeRunCommand();
            break;
          case 'config':
            await executeConfigCommand();
            break;
          case 'trajectory':
            await executeTrajectoryCommand();
            break;
          case 'tools':
            await executeToolsCommand();
            break;
          default:
            setResult('Unknown command. Use --help for available commands.');
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error occurred');
      } finally {
        setIsProcessing(false);
        // Auto-exit after showing result for a moment
        setTimeout(() => {
          exit();
        }, 3000);
      }
    };

    executeCommand();
  }, [args, exit]);

  const executeRunCommand = async () => {
    if (!args.task) {
      throw new Error('Task is required for run command');
    }

    // Simulate task execution
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    setResult(`Task executed successfully: "${args.task}"

This is a placeholder result. The actual agent integration is coming soon!

Configuration used:
• Provider: ${config.llm.provider}
• Model: ${config.llm.model}
• Max Steps: ${config.max_steps}
• Working Directory: ${config.working_directory || process.cwd()}`);
  };

  const executeConfigCommand = async () => {
    // Simulate config operation
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    const action = args.command?.split(' ')[1] || 'show';
    
    switch (action) {
      case 'show':
        setResult(`Current Configuration:
• Provider: ${config.llm.provider}
• Model: ${config.llm.model}
• API Key: ${config.llm.api_key ? '***' : 'Not set'}
• Base URL: ${config.llm.base_url || 'Default'}
• Max Tokens: ${config.llm.max_tokens}
• Temperature: ${config.llm.temperature}
• Max Steps: ${config.max_steps}
• Working Directory: ${config.working_directory || process.cwd()}
• Trajectory Directory: ${config.trajectory_directory}
• Verbose: ${config.verbose ? 'Enabled' : 'Disabled'}
• Theme: ${config.ui?.theme || 'default'}
• Animations: ${config.ui?.animations ? 'Enabled' : 'Disabled'}`);
        break;
      case 'validate':
        setResult('Configuration is valid ✅');
        break;
      case 'init':
        setResult(`Configuration file created: ${args.configFile}
You can now edit this file to customize your settings.`);
        break;
      default:
        setResult('Available config actions: show, validate, init');
    }
  };

  const executeTrajectoryCommand = async () => {
    // Simulate trajectory operation
    await new Promise(resolve => setTimeout(resolve, 1500));
    
    setResult(`Trajectory command executed.

This is a placeholder for trajectory management functionality.
Available actions: list, show, stats, analyze`);
  };

  const executeToolsCommand = async () => {
    // Simulate tools listing
    await new Promise(resolve => setTimeout(resolve, 500));
    
    setResult(`Available Tools:

📁 file_ops - File operations (read, write, create, delete)
⚙️  process - Process management (run commands, manage processes)
📋 task_mgmt - Task management (create, update, track tasks)
🔧 utils - Utility functions (text processing, data manipulation)

Each tool provides specific capabilities for software engineering tasks.
Tools can be enabled/disabled in your configuration file.`);
  };

  if (error) {
    return (
      <Box flexGrow={1} padding={1}>
        <ErrorDisplay error={error} />
      </Box>
    );
  }

  if (isProcessing) {
    return (
      <Box flexGrow={1} justifyContent="center" alignItems="center">
        <LoadingIndicator 
          message={`Executing ${args.command || 'command'}...`}
          showTimer={true}
        />
      </Box>
    );
  }

  return (
    <Box flexGrow={1} padding={1}>
      <Box flexDirection="column">
        <Text color={theme.colors.success} bold>
          ✅ Command completed successfully
        </Text>
        <Text> </Text>
        <Text color={theme.colors.text}>
          {result}
        </Text>
        <Text> </Text>
        <Text color={theme.colors.muted} dimColor>
          Exiting in 3 seconds...
        </Text>
      </Box>
    </Box>
  );
};
