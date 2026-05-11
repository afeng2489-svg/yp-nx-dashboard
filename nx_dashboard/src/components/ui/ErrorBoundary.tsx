import { Component, type ErrorInfo, type ReactNode } from 'react';
import { ErrorState } from './ErrorState';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error('[ErrorBoundary] Caught error:', error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  private handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="flex items-center justify-center h-[calc(100vh-200px)] px-6">
          <ErrorState
            variant="error"
            title="页面崩溃"
            message={this.state.error?.message || '渲染过程中发生未知错误'}
            hints={['组件状态异常', '数据格式不匹配', '依赖服务不可用']}
            actions={[
              { label: '重试', onClick: this.handleReset, variant: 'primary' },
              { label: '刷新页面', onClick: this.handleReload },
            ]}
          />
        </div>
      );
    }

    return this.props.children;
  }
}
