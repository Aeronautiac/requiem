// A panel that crashes on odd state shows an error in its own box, and the rest of the screen keeps
// working. The same rule as the core's fold: log it loudly, say so, limp on.
//
// A class because React only offers error boundaries as classes.
import type { ErrorInfo, ReactNode } from "react";
import { Component } from "react";

type Props = { name: string; children: ReactNode };
type State = { error: string | null };

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: unknown): State {
    return { error: String(error) };
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    console.error(`[amane-ui] ${this.props.name} crashed:`, error, info.componentStack);
  }

  render() {
    if (this.state.error === null) return this.props.children;
    return (
      <div className="flex flex-col gap-2 p-3 text-sm">
        <p className="text-danger-text">{this.props.name} failed to render.</p>
        <p className="break-words text-sm text-ink-dim">{this.state.error}</p>
        <button
          type="button"
          className="self-start border border-edge px-2 py-1 text-xs hover:bg-raised"
          onClick={() => this.setState({ error: null })}
        >
          Try again
        </button>
      </div>
    );
  }
}
