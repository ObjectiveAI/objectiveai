import { Component, type ReactNode } from "react";
import { t } from "../strings";
import { Button, Empty } from "./ui";

type Props = { children: ReactNode };
type State = { error: unknown };

/**
 * One page, held apart: a page that throws while it draws shows why,
 * with a way to draw it again, and the rest of the app stays up. Without
 * it React unmounts the whole window, leaving only its background.
 * React catches render errors only in a class component.
 */
export class Fallen extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: unknown): State {
    return { error };
  }

  render() {
    if (this.state.error === null) return this.props.children;
    const said = this.state.error instanceof Error ? this.state.error.message : String(this.state.error);
    return (
      <div className="page">
        <div className="page-inner">
          <Empty title={t.fallen.title} body={t.fallen.body}>
            <p className="muted">
              {t.fallen.said} <span className="mono selectable">{said}</span>
            </p>
            <Button kind="secondary" onClick={() => this.setState({ error: null })}>
              {t.fallen.again}
            </Button>
          </Empty>
        </div>
      </div>
    );
  }
}
