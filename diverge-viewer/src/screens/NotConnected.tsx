import { Empty } from "../components/ui";
import { t } from "../strings";

/** What every daemon-backed screen shows until the page reaches the daemon. */
export function NotConnected(props: { title: string }) {
  return (
    <div className="page">
      <div className="page-inner">
        <h1 className="page-title">{props.title}</h1>
        <Empty title={t.daemon.notConnectedShort} body={t.daemon.notConnected} />
      </div>
    </div>
  );
}
