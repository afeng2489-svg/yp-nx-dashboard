import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './index.css';
import { useThemeStore } from './stores/themeStore';

// Initialize theme on app load
// eslint-disable-next-line react-refresh/only-export-components
function ThemeInitializer() {
  const { theme, setTheme } = useThemeStore();

  useEffect(() => {
    // Ensure theme is applied on mount
    setTheme(theme);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return null;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeInitializer />
    <App />
  </StrictMode>,
);
