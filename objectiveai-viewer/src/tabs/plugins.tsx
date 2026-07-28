import { useEffect, useState } from "react";
import cn from "classnames";
import { tauriListen } from "../lib/tauri";
import {
  pluginsInstall,
  pluginsList,
  pluginsUninstall,
  type PluginVersionInfo,
} from "../lib/plugins";

/** The plugins home tab: install a plugin's viewer extension by its
 * three coordinates (owner / name / v-prefixed git tag), and
 * uninstall any exact installed version. Enable/disable and ordering
 * of the resulting tabs stay in the TABS tab — this surface only
 * changes what is installed on disk.
 *
 * Install and uninstall run inline in Rust (fetch → pnpm → esbuild →
 * land; unload → delete → prune) — the promise resolving IS the
 * pipeline finishing, and the shell emits `inventory://changed` when
 * the tab inventory shifts, which doubles as our refetch signal.
 * Progress detail streams to viewer-logs. */
export default function PluginsTab() {
  const [list, setList] = useState<PluginVersionInfo[]>([]);
  const [owner, setOwner] = useState("");
  const [name, setName] = useState("");
  const [version, setVersion] = useState("");
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);
  const [uninstalling, setUninstalling] = useState<string | null>(null);
  const [uninstallError, setUninstallError] = useState<{
    key: string;
    message: string;
  } | null>(null);

  const refetch = async () => {
    const versions = await pluginsList();
    if (versions) setList(versions);
  };

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      unlisten = await tauriListen("inventory://changed", () => {
        if (disposed) return;
        void refetch();
      });
      if (disposed) {
        unlisten?.();
        return;
      }
      const versions = await pluginsList();
      if (versions && !disposed) setList(versions);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const ready =
    owner.trim() !== "" && name.trim() !== "" && version.trim() !== "";

  const install = () => {
    if (!ready || installing) return;
    setInstalling(true);
    setInstallError(null);
    pluginsInstall(owner.trim(), name.trim(), version.trim())
      .then(() => {
        setOwner("");
        setName("");
        setVersion("");
      })
      .catch((e: unknown) => {
        setInstallError(String(e));
      })
      .finally(() => {
        setInstalling(false);
        void refetch();
      });
  };

  const uninstall = (row: PluginVersionInfo) => {
    const key = rowKey(row);
    if (uninstalling !== null) return;
    setUninstalling(key);
    setUninstallError(null);
    pluginsUninstall(row.owner, row.name, row.version)
      .catch((e: unknown) => {
        setUninstallError({ key, message: String(e) });
      })
      .finally(() => {
        setUninstalling(null);
        void refetch();
      });
  };

  return (
    <div className={cn("flex-1", "min-h-0", "overflow-y-auto", "font-mono")}>
      {/* Install form. */}
      <div
        className={cn(
          "p-4",
          "border-b",
          "border-node-border",
          "flex",
          "flex-col",
          "gap-2",
        )}
      >
        <div
          className={cn(
            "text-[11px]",
            "text-info-dim",
            "uppercase",
            "tracking-wider",
            "select-none",
          )}
        >
          install plugin
        </div>
        <div className={cn("flex", "gap-2", "flex-wrap", "items-center")}>
          <Field
            value={owner}
            placeholder="owner"
            disabled={installing}
            onChange={setOwner}
            onEnter={install}
          />
          <Field
            value={name}
            placeholder="name"
            disabled={installing}
            onChange={setName}
            onEnter={install}
          />
          <Field
            value={version}
            placeholder="v1.2.3"
            disabled={installing}
            onChange={setVersion}
            onEnter={install}
          />
          <button
            disabled={!ready || installing}
            onClick={install}
            className={cn(
              "px-4",
              "py-1",
              "rounded-md",
              "border",
              "border-copper-mid",
              "text-copper-bright",
              "text-xs",
              "uppercase",
              "tracking-wider",
              installing
                ? cn("opacity-50", "cursor-wait")
                : ready
                  ? cn(
                      "cursor-pointer",
                      "hover:bg-ground-surface",
                      "hover:border-copper-bright",
                    )
                  : cn("opacity-40"),
              "transition-colors",
            )}
          >
            {installing ? "installing…" : "install"}
          </button>
        </div>
        {installError !== null && (
          <div className={cn("text-[11px]", "text-error", "break-all")}>
            {installError}
          </div>
        )}
      </div>

      {/* Installed versions. */}
      {list.length === 0 && (
        <div
          className={cn(
            "p-6",
            "text-[11px]",
            "text-info-dim",
            "select-none",
          )}
        >
          no plugins installed
        </div>
      )}
      {list.map((row) => {
        const key = rowKey(row);
        const busy = uninstalling === key;
        return (
          <div
            key={key}
            className={cn(
              "px-4",
              "py-2",
              "border-b",
              "border-node-border",
              "flex",
              "flex-col",
              "gap-1",
            )}
          >
            <div className={cn("flex", "items-center", "gap-3")}>
              <span className={cn("text-xs", "text-info-bright")}>
                {row.owner}/{row.name}
              </span>
              <span className={cn("text-xs", "text-copper-bright")}>
                {row.version}
              </span>
              {!row.hasViewer && (
                <span className={cn("text-[11px]", "text-info-dim")}>
                  (no viewer extension)
                </span>
              )}
              <span className={cn("flex-1")} />
              <button
                disabled={uninstalling !== null}
                onClick={() => uninstall(row)}
                className={cn(
                  "px-3",
                  "py-0.5",
                  "rounded-md",
                  "border",
                  "border-node-border",
                  "text-info-dim",
                  "text-[11px]",
                  "uppercase",
                  "tracking-wider",
                  busy
                    ? cn("opacity-50", "cursor-wait")
                    : uninstalling !== null
                      ? cn("opacity-40")
                      : cn(
                          "cursor-pointer",
                          "hover:text-error",
                          "hover:bg-ground-surface",
                          "hover:border-error",
                        ),
                  "transition-colors",
                )}
              >
                {busy ? "uninstalling…" : "uninstall"}
              </button>
            </div>
            {row.description !== undefined && (
              <div className={cn("text-[11px]", "text-info-dim", "truncate")}>
                {row.description}
              </div>
            )}
            {uninstallError !== null && uninstallError.key === key && (
              <div
                className={cn("text-[11px]", "text-error", "break-all")}
              >
                {uninstallError.message}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function rowKey(row: PluginVersionInfo): string {
  return `${row.owner}/${row.name}/${row.version}`;
}

function Field(props: {
  value: string;
  placeholder: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onEnter: () => void;
}) {
  return (
    <input
      value={props.value}
      placeholder={props.placeholder}
      disabled={props.disabled}
      onChange={(e) => props.onChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") props.onEnter();
      }}
      spellCheck={false}
      className={cn(
        "bg-ground-surface",
        "border",
        "border-copper-dim",
        "rounded-sm",
        "px-2",
        "py-1",
        "text-xs",
        "text-info-bright",
        "font-medium",
        "outline-none",
        "focus:border-copper-bright",
        "placeholder:text-info-dim",
        props.disabled && cn("opacity-50"),
        "w-40",
      )}
    />
  );
}
