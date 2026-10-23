import { NotConnected } from "./NotConnected";
import { t } from "../strings";

// Until the page reaches the daemon: the screen's name, and why there is nothing in it.
export function Conversation(props: { template: string; index: number; name: string | null }) {
  return <NotConnected title={props.name ?? `${t.rail.unnamed} ${props.index}`} />;
}
