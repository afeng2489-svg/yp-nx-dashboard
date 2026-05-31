/** @type {import('tailwindcss').Config} */
// 设计系统接线层：颜色/字体/圆角/阴影全部引用 src/styles/tokens.css 里的 CSS 变量。
// 想换整站气质，只改 tokens.css，不用动组件。
export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: 'hsl(var(--bg) / <alpha-value>)',
        surface: 'hsl(var(--surface) / <alpha-value>)',
        elevated: 'hsl(var(--elevated) / <alpha-value>)',
        border: 'hsl(var(--border) / <alpha-value>)',
        ink: 'hsl(var(--ink) / <alpha-value>)',
        muted: 'hsl(var(--muted) / <alpha-value>)',
        primary: {
          DEFAULT: 'hsl(var(--primary) / <alpha-value>)',
          foreground: 'hsl(var(--primary-fg) / <alpha-value>)',
        },
        accent: {
          DEFAULT: 'hsl(var(--accent) / <alpha-value>)',
          foreground: 'hsl(var(--accent-fg) / <alpha-value>)',
        },
      },
      fontFamily: {
        display: 'var(--font-display)',
        sans: 'var(--font-sans)',
      },
      borderRadius: {
        sm: 'var(--radius-sm)',
        DEFAULT: 'var(--radius)',
        lg: 'var(--radius-lg)',
        xl: 'var(--radius-xl)',
        '2xl': 'var(--radius-2xl)',
      },
      boxShadow: {
        soft: 'var(--shadow-soft)',
        lift: 'var(--shadow-lift)',
        glow: 'var(--shadow-glow)',
      },
      maxWidth: {
        content: 'var(--content-width)',
      },
      keyframes: {
        'fade-up': {
          from: { opacity: '0', transform: 'translateY(16px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-10px)' },
        },
      },
      animation: {
        'fade-up': 'fade-up 0.7s cubic-bezier(0.16, 1, 0.3, 1) both',
        float: 'float 6s ease-in-out infinite',
      },
    },
  },
  plugins: [],
};
