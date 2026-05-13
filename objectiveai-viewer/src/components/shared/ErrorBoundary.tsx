import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center py-10 px-6 text-center">
          <div className="text-info-dim text-sm mb-3">Something went wrong rendering this entry.</div>
          <button
            className="font-mono text-[10px] text-info-dim bg-ground-raised border border-node-border rounded-sm px-3 py-1 hover:text-info-bright transition-colors"
            onClick={() => this.setState({ hasError: false })}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
