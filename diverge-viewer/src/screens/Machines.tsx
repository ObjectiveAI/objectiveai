import { NotConnected } from "./NotConnected";
import { t } from "../strings";

// Until the page reaches the daemon: the screen's name, and why there is nothing in it.
export function Machines() {
  return <NotConnected title={t.machines.title} />;
}
