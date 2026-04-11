import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ThemeToggle } from '../ui/ThemeToggle'
import { useThemeStore } from '@/stores/themeStore'

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Moon: ({ className }: { className?: string }) => (
    <span data-testid="moon-icon" className={className}>Moon</span>
  ),
  Sun: ({ className }: { className?: string }) => (
    <span data-testid="sun-icon" className={className}>Sun</span>
  ),
  Monitor: ({ className }: { className?: string }) => (
    <span data-testid="monitor-icon" className={className}>Monitor</span>
  ),
}))

describe('ThemeToggle', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Reset store state
    useThemeStore.setState({
      theme: 'system',
      resolvedTheme: 'light',
    })
  })

  afterEach(() => {
    // Clean up document classes
    document.documentElement.classList.remove('light', 'dark')
  })

  it('should render with system theme by default', () => {
    render(<ThemeToggle />)
    expect(screen.getByTestId('monitor-icon')).toBeInTheDocument()
  })

  it('should render with dark theme icon when dark', () => {
    useThemeStore.setState({ theme: 'dark', resolvedTheme: 'dark' })
    render(<ThemeToggle />)
    expect(screen.getByTestId('moon-icon')).toBeInTheDocument()
  })

  it('should render with light theme icon when light', () => {
    useThemeStore.setState({ theme: 'light', resolvedTheme: 'light' })
    render(<ThemeToggle />)
    expect(screen.getByTestId('sun-icon')).toBeInTheDocument()
  })

  it('should cycle theme on click', () => {
    render(<ThemeToggle />)

    const button = screen.getByRole('button')

    // Initial: system
    expect(useThemeStore.getState().theme).toBe('system')

    // Click -> light
    fireEvent.click(button)
    expect(useThemeStore.getState().theme).toBe('light')

    // Click -> dark
    fireEvent.click(button)
    expect(useThemeStore.getState().theme).toBe('dark')

    // Click -> system (cycles back)
    fireEvent.click(button)
    expect(useThemeStore.getState().theme).toBe('system')
  })

  it('should apply theme class to document', () => {
    render(<ThemeToggle />)

    const button = screen.getByRole('button')

    fireEvent.click(button) // light
    expect(document.documentElement.classList.contains('light')).toBe(true)

    fireEvent.click(button) // dark
    expect(document.documentElement.classList.contains('dark')).toBe(true)

    fireEvent.click(button) // system
    // System resolves to whatever matchMedia returns (mocked to false in setup)
    expect(document.documentElement.classList.contains('light')).toBe(true)
  })

  it('should pass className prop', () => {
    const { container } = render(<ThemeToggle className="custom-class" />)
    const wrapper = container.firstChild
    expect(wrapper).toHaveClass('custom-class')
  })

  it('should have correct title with theme label', () => {
    useThemeStore.setState({ theme: 'dark' })
    render(<ThemeToggle />)
    expect(screen.getByRole('button')).toHaveAttribute('title', '当前: 深色')
  })
})
