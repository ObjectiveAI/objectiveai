type Listener = (event: ViewerEvent) => void;

export interface ViewerEvent {
  kind: "agent-completions" | "functions-executions" | "functions-inventions-recursive" | "laboratories-executions";
  payload: unknown;
  timestamp: number;
}

const listeners = new Set<Listener>();
const buffer: ViewerEvent[] = [];
const MAX_BUFFER = 500;

export function pushEvent(event: ViewerEvent) {
  buffer.push(event);
  if (buffer.length > MAX_BUFFER) buffer.shift();
  for (const fn of listeners) {
    fn(event);
  }
}

export function subscribe(fn: Listener): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function getBuffer(): ViewerEvent[] {
  return [...buffer];
}

export function clearBuffer() {
  buffer.length = 0;
}
