#!/usr/bin/env node

/**
 * Sage Agent CLI - Modern terminal UI with Ink and React
 * 
 * This is the main entry point for the Sage Agent CLI application.
 * It replaces the previous Rust-based CLI with a modern Node.js + Ink + React implementation.
 */

import React from 'react';
import { render } from 'ink';
import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';
import { App } from './components/App.js';
import { SageConfig } from './types/config.js';
import { loadConfig } from './utils/config.js';

interface CliArgs {
  command?: string;
  task?: string;
  provider?: string;
  model?: string;
  modelBaseUrl?: string;
  apiKey?: string;
  maxSteps?: number;
  workingDir?: string;
  configFile: string;
  trajectoryFile?: string;
  patchPath?: string;
  mustPatch?: boolean;
  verbose?: boolean;
  force?: boolean;
  directory?: string;
}

async function main() {
  const argv = await yargs(hideBin(process.argv))
    .scriptName('sage')
    .usage('$0 [command] [options]')
    .command('run <task>', 'Run a task using Sage Agent', (yargs) => {
      return yargs
        .positional('task', {
          describe: 'The task description or path to a file containing the task',
          type: 'string',
          demandOption: true,
        })
        .option('provider', {
          describe: 'LLM provider to use (openai, anthropic, google, ollama)',
          type: 'string',
        })
        .option('model', {
          describe: 'Model to use',
          type: 'string',
        })
        .option('model-base-url', {
          describe: 'Base URL for the model API',
          type: 'string',
        })
        .option('api-key', {
          describe: 'API key for the provider',
          type: 'string',
        })
        .option('max-steps', {
          describe: 'Maximum number of execution steps',
          type: 'number',
        })
        .option('working-dir', {
          describe: 'Working directory for the agent',
          type: 'string',
        })
        .option('trajectory-file', {
          describe: 'Path to save trajectory file',
          type: 'string',
        })
        .option('patch-path', {
          describe: 'Path to patch file',
          type: 'string',
        })
        .option('must-patch', {
          describe: 'Whether to create a patch',
          type: 'boolean',
          default: false,
        });
    })
    .command('interactive', 'Interactive mode', (yargs) => {
      return yargs
        .option('trajectory-file', {
          describe: 'Path to save trajectory file',
          type: 'string',
        })
        .option('working-dir', {
          describe: 'Working directory for the agent',
          type: 'string',
        });
    })
    .command('config <action>', 'Configuration management', (yargs) => {
      return yargs
        .positional('action', {
          describe: 'Configuration action',
          choices: ['show', 'validate', 'init'],
          demandOption: true,
        })
        .option('force', {
          describe: 'Overwrite existing file (for init action)',
          type: 'boolean',
          default: false,
        });
    })
    .command('trajectory <action> [path]', 'Trajectory management', (yargs) => {
      return yargs
        .positional('action', {
          describe: 'Trajectory action',
          choices: ['list', 'show', 'stats', 'analyze'],
          demandOption: true,
        })
        .positional('path', {
          describe: 'Path to trajectory file or directory',
          type: 'string',
        })
        .option('directory', {
          describe: 'Directory to search for trajectories (for list action)',
          type: 'string',
          default: '.',
        });
    })
    .command('tools', 'Show available tools and their descriptions')
    .option('config-file', {
      describe: 'Path to configuration file',
      type: 'string',
      default: 'sage_config.json',
    })
    .option('verbose', {
      describe: 'Enable verbose output',
      type: 'boolean',
      alias: 'v',
      default: false,
    })
    .help()
    .alias('help', 'h')
    .version()
    .alias('version', 'V')
    .parse() as CliArgs;

  // Load configuration
  let config: SageConfig;
  try {
    config = await loadConfig(argv.configFile);
  } catch (error) {
    console.error(`Failed to load config: ${error}`);
    process.exit(1);
  }

  // Check if raw mode is supported (required for interactive mode)
  const isRawModeSupported = process.stdin.isTTY && typeof process.stdin.setRawMode === 'function';

  // Determine if we should run in interactive mode
  const shouldBeInteractive =
    isRawModeSupported && (
      !argv.command ||
      argv.command === 'interactive' ||
      !argv.task
    );

  if (!isRawModeSupported && (argv.command === 'interactive' || !argv.command)) {
    console.error('Error: Interactive mode requires a TTY with raw mode support.');
    console.error('Please run this command in a real terminal, not through a non-interactive shell.');
    console.error('');
    console.error('Alternatively, you can use the Rust CLI:');
    console.error('  sage interactive --claude-style');
    process.exit(1);
  }

  if (shouldBeInteractive) {
    // Render the Ink UI
    const { unmount } = render(
      React.createElement(App, {
        config,
        args: argv,
        mode: 'interactive'
      }),
      { exitOnCtrlC: false }
    );

    // Handle cleanup
    process.on('SIGINT', () => {
      unmount();
      process.exit(0);
    });

    process.on('SIGTERM', () => {
      unmount();
      process.exit(0);
    });
  } else {
    // Non-interactive mode - render appropriate command UI
    const { unmount } = render(
      React.createElement(App, {
        config,
        args: argv,
        mode: 'command'
      }),
      { exitOnCtrlC: false }
    );

    // Handle cleanup
    process.on('SIGINT', () => {
      unmount();
      process.exit(0);
    });

    process.on('SIGTERM', () => {
      unmount();
      process.exit(0);
    });
  }
}

// Handle unhandled promise rejections
process.on('unhandledRejection', (reason, promise) => {
  console.error('Unhandled Rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

// Handle uncaught exceptions
process.on('uncaughtException', (error) => {
  console.error('Uncaught Exception:', error);
  process.exit(1);
});

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
