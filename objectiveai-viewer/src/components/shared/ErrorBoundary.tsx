import { Component, type ReactNode, type ErrorInfo } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center py-10 px-6 text-center">
          <div className="text-info-dim text-sm mb-1">Something went wrong rendering this entry.</div>
          <div className="text-error text-xs font-mono mb-3 max-w-md truncate">{this.state.error?.message}</div>
          <button
            className="font-mono text-[10px] text-info-dim bg-ground-raised border border-node-border rounded-sm px-3 py-1 hover:text-info-bright transition-colors"
            onClick={() => this.setState({ hasError: false, error: null })}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
