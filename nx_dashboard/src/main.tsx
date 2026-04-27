import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './index.css';
import { useThemeStore } from './stores/themeStore';

// Initialize theme on app load
function ThemeInitializer() {
  const { theme, setTheme } = useThemeStore();

  useEffect(() => {
    // Ensure theme is applied on mount
    setTheme(theme);
  }, []);

  return null;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeInitializer />
    <App />
  </StrictMode>,
);
