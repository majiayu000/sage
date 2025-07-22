/**
 * Theme context for managing UI themes
 */

import React, { createContext, useContext, ReactNode } from 'react';

export interface Theme {
  name: string;
  colors: {
    primary: string;
    secondary: string;
    accent: string;
    success: string;
    warning: string;
    error: string;
    info: string;
    text: string;
    muted: string;
    border: string;
    background: string;
  };
}

const themes: Record<string, Theme> = {
  default: {
    name: 'default',
    colors: {
      primary: 'blue',
      secondary: 'cyan',
      accent: 'magenta',
      success: 'green',
      warning: 'yellow',
      error: 'red',
      info: 'blue',
      text: 'white',
      muted: 'gray',
      border: 'gray',
      background: 'black',
    },
  },
  dark: {
    name: 'dark',
    colors: {
      primary: 'blueBright',
      secondary: 'cyanBright',
      accent: 'magentaBright',
      success: 'greenBright',
      warning: 'yellowBright',
      error: 'redBright',
      info: 'blueBright',
      text: 'whiteBright',
      muted: 'gray',
      border: 'gray',
      background: 'black',
    },
  },
  light: {
    name: 'light',
    colors: {
      primary: 'blue',
      secondary: 'cyan',
      accent: 'magenta',
      success: 'green',
      warning: 'yellow',
      error: 'red',
      info: 'blue',
      text: 'black',
      muted: 'gray',
      border: 'gray',
      background: 'white',
    },
  },
  ocean: {
    name: 'ocean',
    colors: {
      primary: 'cyan',
      secondary: 'blue',
      accent: 'blueBright',
      success: 'green',
      warning: 'yellow',
      error: 'red',
      info: 'cyan',
      text: 'white',
      muted: 'gray',
      border: 'cyan',
      background: 'black',
    },
  },
  forest: {
    name: 'forest',
    colors: {
      primary: 'green',
      secondary: 'greenBright',
      accent: 'yellow',
      success: 'green',
      warning: 'yellow',
      error: 'red',
      info: 'green',
      text: 'white',
      muted: 'gray',
      border: 'green',
      background: 'black',
    },
  },
};

const ThemeContext = createContext<Theme>(themes.default);

interface ThemeProviderProps {
  theme: string;
  children: ReactNode;
}

export const ThemeProvider: React.FC<ThemeProviderProps> = ({ theme, children }) => {
  const selectedTheme = themes[theme] || themes.default;
  
  return (
    <ThemeContext.Provider value={selectedTheme}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = (): Theme => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};

export const getAvailableThemes = (): string[] => {
  return Object.keys(themes);
};

export const getTheme = (name: string): Theme => {
  return themes[name] || themes.default;
};
