/**
 * Configuration loading and management utilities
 */

import { readFile, access } from 'fs/promises';
import { constants } from 'fs';
import { SageConfig } from '../types/config.js';

/**
 * Default configuration
 */
const DEFAULT_CONFIG: SageConfig = {
  llm: {
    provider: 'openai',
    model: 'gpt-4',
    max_tokens: 4000,
    temperature: 0.1,
    timeout: 30000,
  },
  tools: [
    { name: 'file_ops', enabled: true },
    { name: 'process', enabled: true },
    { name: 'task_mgmt', enabled: true },
    { name: 'utils', enabled: true },
  ],
  max_steps: 50,
  working_directory: process.cwd(),
  trajectory_directory: './trajectories',
  verbose: false,
  ui: {
    theme: 'default',
    animations: true,
    colors: true,
  },
};

/**
 * Load configuration from file
 */
export async function loadConfig(configPath: string): Promise<SageConfig> {
  try {
    // Check if config file exists
    await access(configPath, constants.F_OK);
    
    // Read and parse config file
    const configContent = await readFile(configPath, 'utf-8');
    const userConfig = JSON.parse(configContent);
    
    // Merge with default config
    return mergeConfig(DEFAULT_CONFIG, userConfig);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      console.warn(`Config file ${configPath} not found, using default configuration`);
      return DEFAULT_CONFIG;
    }
    throw new Error(`Failed to load config from ${configPath}: ${error}`);
  }
}

/**
 * Merge user config with default config
 */
function mergeConfig(defaultConfig: SageConfig, userConfig: Partial<SageConfig>): SageConfig {
  return {
    ...defaultConfig,
    ...userConfig,
    llm: {
      ...defaultConfig.llm,
      ...userConfig.llm,
    },
    tools: userConfig.tools || defaultConfig.tools,
    ui: {
      ...defaultConfig.ui,
      ...userConfig.ui,
    },
  };
}

/**
 * Validate configuration
 */
export function validateConfig(config: SageConfig): string[] {
  const errors: string[] = [];

  // Validate LLM config
  if (!config.llm.provider) {
    errors.push('LLM provider is required');
  }

  if (!config.llm.model) {
    errors.push('LLM model is required');
  }

  if (!['openai', 'anthropic', 'google', 'ollama'].includes(config.llm.provider)) {
    errors.push('Invalid LLM provider. Must be one of: openai, anthropic, google, ollama');
  }

  // Validate max_steps
  if (config.max_steps && (config.max_steps < 1 || config.max_steps > 1000)) {
    errors.push('max_steps must be between 1 and 1000');
  }

  // Validate tools
  if (!Array.isArray(config.tools)) {
    errors.push('tools must be an array');
  }

  return errors;
}

/**
 * Get environment variable with fallback
 */
export function getEnvVar(name: string, fallback?: string): string | undefined {
  return process.env[name] || fallback;
}

/**
 * Apply environment variable overrides to config
 */
export function applyEnvOverrides(config: SageConfig): SageConfig {
  const envConfig = { ...config };

  // Override LLM settings from environment
  if (process.env.SAGE_LLM_PROVIDER) {
    envConfig.llm.provider = process.env.SAGE_LLM_PROVIDER as any;
  }

  if (process.env.SAGE_LLM_MODEL) {
    envConfig.llm.model = process.env.SAGE_LLM_MODEL;
  }

  if (process.env.SAGE_API_KEY || process.env.OPENAI_API_KEY || process.env.ANTHROPIC_API_KEY) {
    envConfig.llm.api_key = 
      process.env.SAGE_API_KEY || 
      process.env.OPENAI_API_KEY || 
      process.env.ANTHROPIC_API_KEY;
  }

  if (process.env.SAGE_LLM_BASE_URL) {
    envConfig.llm.base_url = process.env.SAGE_LLM_BASE_URL;
  }

  if (process.env.SAGE_MAX_STEPS) {
    envConfig.max_steps = parseInt(process.env.SAGE_MAX_STEPS, 10);
  }

  if (process.env.SAGE_WORKING_DIR) {
    envConfig.working_directory = process.env.SAGE_WORKING_DIR;
  }

  if (process.env.SAGE_VERBOSE) {
    envConfig.verbose = process.env.SAGE_VERBOSE === 'true';
  }

  return envConfig;
}
