import type { ProviderView } from "../bindings/ProviderView";
import { t } from "../strings";

let machineNames: Record<string, string> = {};
export function setMachineNames(names: Record<string, string>) {
  machineNames = names;
}
export function identityKey(p: ProviderView): string {
  return p.kind === "outgoing" ? `outgoing:${p.address}` : `incoming:${p.identity}`;
}
/** The daemon's own name for a provider: an address it dials, or an identity it judged. */
export function providerRaw(p: ProviderView): string {
  return p.kind === "outgoing" ? p.address : p.identity;
}
/** What you call it, if you named it; the daemon's name otherwise. */
export function providerName(p: ProviderView): string {
  return machineNames[identityKey(p)] ?? providerRaw(p);
}

export function providerWay(p: ProviderView): string {
  return p.kind === "outgoing" ? t.provider.outgoing : t.provider.incoming;
}

export function kindOf(imageName: string): string {
  return imageName.replace(/^diverge-agentic-loop-/, "");
}

export function kindTitle(imageName: string): string {
  const key = kindOf(imageName);
  return t.create.kinds[key]?.title ?? key;
}

export function time(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const sameDay = d.toDateString() === now.toDateString();
  return sameDay
    ? d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })
    : d.toLocaleString([], { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
}

export function ago(iso: string): string {
  const s = Math.max(0, (Date.now() - new Date(iso).getTime()) / 1000);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 ** 3).toFixed(n % 1024 ** 3 === 0 ? 0 : 1)} GB`;
}

export function number(n: number): string {
  return n.toLocaleString();
}
