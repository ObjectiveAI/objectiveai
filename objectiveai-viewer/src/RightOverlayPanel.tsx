import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import cn from "classnames";
import type {
  AgentCompletionsMessageMessage,
  AgentCompletionsMessageRichContent,
  AgentCompletionsMessageRichContentPart,
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  ErrorResponseError,
  RemotePathCommitOptional,
} from "@objectiveai/sdk";
import { ChatPane } from "./ChatPane";
import { MessageList } from "./chat/MessageList";

/** Human-readable label for an agent's remote path, used as the tab
 * title and chat header. */
export function agentLabel(agent: RemotePathCommitOptional): string {
  if ("name" in agent) return agent.name;
  return `${agent.owner}/${agent.repository}`;
}

export interface PanelTabUserEntry {
  kind: "user";
  message: AgentCompletionsMessageMessage;
}

export interface PanelTabCompletionEntry {
  kind: "completion";
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  streaming: boolean;
  error: ErrorResponseError | null;
  /** Per-completion uuid used as the cli_run channel name. Distinct
   * from `chunk.id` (server-assigned response_id used for notify). */
  requestId: string;
}

export type PanelTabEntry = PanelTabUserEntry | PanelTabCompletionEntry;

export interface PanelTab {
  id: string;
  agent: RemotePathCommitOptional;
  draft: string;
  attachments: AgentCompletionsMessageRichContentPart[];
  /** Chronologically-ordered chat entries. Render in order. */
  entries: PanelTabEntry[];
  /** Opaque continuation, from the most recent completion entry's
   * final chunk. */
  continuation: string | null;
  /** Index into `entries` of the currently-streaming completion
   * entry (which has `kind: "completion"` and `streaming: true`),
   * or `null` when idle. */
  inFlightEntryIndex: number | null;
  /** RichContent payloads waiting on the in-flight completion's
   * `chunk.id` so the notify call can carry a `response_id`. The
   * first-chunk handler flushes them. Empty when idle. */
  pendingNotifyContent: AgentCompletionsMessageRichContent[];
  /** Every user message ever sent mid-flight via notify, in
   * insertion order. Rendered as DIMMED user bubbles at the
   * BOTTOM of the chat (after all entries) — the "visual queue".
   * As tool responses arrive carrying embedded
   * `<system-reminder>` user notifications, the matching prefix
   * is "absorbed" (FIFO) and those messages render in their
   * natural position below the tool response. */
  notifyHistory: AgentCompletionsMessageMessage[];
  /** Open/closed overrides for collapsibles (reasoning, tool
   * calls, tool responses). Keyed via the helpers in
   * `chat/dropdownKey.ts`. Absence = use default state. User
   * clicks set entries here; persists for the lifetime of the
   * panel tab so toggling the panel / switching tabs preserves
   * the user's preference. */
  dropdownOverrides: Record<string, boolean>;
}

interface RightOverlayPanelProps {
  panelTabs: PanelTab[];
  setPanelTabs: Dispatch<SetStateAction<PanelTab[]>>;
  activePanelTabId: string | null;
  setActivePanelTabId: Dispatch<SetStateAction<string | null>>;
  sendMessage: (tabId: string) => void;
}

export function RightOverlayPanel({
  panelTabs,
  setPanelTabs,
  activePanelTabId,
  setActivePanelTabId,
  sendMessage,
}: RightOverlayPanelProps) {
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const plusRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!dropdownOpen) return;
    const onDocDown = (e: MouseEvent) => {
      if (plusRef.current && !plusRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocDown);
    return () => document.removeEventListener("mousedown", onDocDown);
  }, [dropdownOpen]);

  const openAgent = (agent: RemotePathCommitOptional) => {
    const id = crypto.randomUUID();
    const tab: PanelTab = {
      id,
      agent,
      draft: "",
      attachments: [],
      entries: [],
      continuation: null,
      inFlightEntryIndex: null,
      pendingNotifyContent: [],
      notifyHistory: [],
      dropdownOverrides: {},
    };
    setPanelTabs((prev) => [...prev, tab]);
    setActivePanelTabId(id);
    setDropdownOpen(false);
  };

  const closeTab = (id: string) => {
    setPanelTabs((prev) => {
      const next = prev.filter((t) => t.id !== id);
      if (activePanelTabId === id) {
        setActivePanelTabId(next.length > 0 ? next[next.length - 1].id : null);
      }
      return next;
    });
  };

  const updateTab = (id: string, patch: Partial<PanelTab>) => {
    setPanelTabs((prev) =>
      prev.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    );
  };

  const activeTab = panelTabs.find((t) => t.id === activePanelTabId) ?? null;

  return (
    <div
      className={cn(
        "absolute",
        "top-0",
        "right-0",
        "bottom-0",
        "w-96",
        "z-10",
        "bg-white",
        "dark:bg-neutral-950",
        "border-l",
        "border-neutral-300",
        "dark:border-neutral-700",
        "shadow-lg",
        "flex",
        "flex-col",
      )}
      role="complementary"
    >
      <nav
        role="tablist"
        className={cn(
          "flex",
          "flex-row",
          "items-stretch",
          "gap-1",
          "px-2",
          "pt-2",
          "bg-neutral-100",
          "dark:bg-neutral-900",
          "border-b",
          "border-neutral-300",
          "dark:border-neutral-700",
          "overflow-x-auto",
        )}
      >
        {panelTabs.map((tab) => {
          const isActive = tab.id === activePanelTabId;
          return (
            <div
              key={tab.id}
              className={cn(
                "flex",
                "flex-row",
                "items-center",
                "rounded-t-md",
                "border-b-2",
                isActive
                  ? cn(
                      "bg-white",
                      "border-blue-600",
                      "dark:bg-neutral-800",
                      "dark:border-blue-400",
                    )
                  : cn(
                      "bg-transparent",
                      "border-transparent",
                      "hover:bg-neutral-200",
                      "dark:hover:bg-neutral-800",
                    ),
              )}
            >
              <button
                type="button"
                role="tab"
                aria-selected={isActive}
                onClick={() => setActivePanelTabId(tab.id)}
                className={cn(
                  "px-3",
                  "py-2",
                  "text-sm",
                  "cursor-pointer",
                  "max-w-40",
                  "truncate",
                  isActive
                    ? cn(
                        "text-neutral-900",
                        "dark:text-neutral-50",
                        "font-semibold",
                      )
                    : cn("text-neutral-600", "dark:text-neutral-400"),
                )}
              >
                {agentLabel(tab.agent)}
              </button>
              <button
                type="button"
                onClick={() => closeTab(tab.id)}
                aria-label={`Close ${agentLabel(tab.agent)}`}
                className={cn(
                  "pr-2",
                  "py-1",
                  "text-neutral-500",
                  "dark:text-neutral-400",
                  "hover:text-neutral-900",
                  "dark:hover:text-neutral-50",
                  "cursor-pointer",
                  "text-sm",
                )}
              >
                {"×"}
              </button>
            </div>
          );
        })}

        <div ref={plusRef} className={cn("relative", "flex", "items-stretch")}>
          <button
            type="button"
            onClick={() => setDropdownOpen((v) => !v)}
            aria-label="Open agent"
            className={cn(
              "px-3",
              "py-2",
              "rounded-t-md",
              "border-b-2",
              "border-transparent",
              "text-sm",
              "text-neutral-600",
              "dark:text-neutral-400",
              "hover:bg-neutral-200",
              "dark:hover:bg-neutral-800",
              "cursor-pointer",
            )}
          >
            {"+"}
          </button>

          {dropdownOpen && <NewAgentForm onOpen={openAgent} />}
        </div>
      </nav>

      <div className={cn("flex", "flex-col", "flex-1", "min-h-0")}>
        {activeTab ? (
          <ActiveTabBody
            tab={activeTab}
            setPanelTabs={setPanelTabs}
            onDraftChange={(draft) => updateTab(activeTab.id, { draft })}
            onAttachmentsChange={(attachments) =>
              updateTab(activeTab.id, { attachments })
            }
            onSend={() => sendMessage(activeTab.id)}
          />
        ) : (
          <div
            className={cn(
              "flex",
              "flex-1",
              "items-center",
              "justify-center",
              "px-4",
              "text-center",
              "text-sm",
              "text-neutral-500",
              "dark:text-neutral-400",
            )}
          >
            Open an agent from + to start a conversation.
          </div>
        )}
      </div>
    </div>
  );
}

interface NewAgentFormProps {
  onOpen: (agent: RemotePathCommitOptional) => void;
}

/** Form to start a chat with a GitHub-hosted agent by `owner`,
 * `repository`, and optional `commit`. Replaces the removed favorites
 * list as the agent-selection source. */
function NewAgentForm({ onOpen }: NewAgentFormProps) {
  const [owner, setOwner] = useState("");
  const [repository, setRepository] = useState("");
  const [commit, setCommit] = useState("");

  const canSubmit = owner.trim().length > 0 && repository.trim().length > 0;

  const submit = () => {
    if (!canSubmit) return;
    onOpen({
      remote: "github",
      owner: owner.trim(),
      repository: repository.trim(),
      commit: commit.trim() === "" ? null : commit.trim(),
    });
  };

  const inputClass = cn(
    "w-full",
    "px-2",
    "py-1.5",
    "rounded",
    "border",
    "border-neutral-300",
    "dark:border-neutral-700",
    "bg-white",
    "dark:bg-neutral-950",
    "text-sm",
    "text-neutral-900",
    "dark:text-neutral-50",
    "outline-none",
    "focus:border-blue-500",
  );

  return (
    <div
      role="menu"
      className={cn(
        "absolute",
        "top-full",
        "left-0",
        "mt-1",
        "min-w-64",
        "rounded-md",
        "border",
        "border-neutral-300",
        "dark:border-neutral-700",
        "bg-white",
        "dark:bg-neutral-900",
        "shadow-lg",
        "z-20",
        "p-3",
        "flex",
        "flex-col",
        "gap-2",
      )}
    >
      <input
        type="text"
        value={owner}
        onChange={(e) => setOwner(e.target.value)}
        placeholder="owner"
        aria-label="Agent owner"
        className={inputClass}
      />
      <input
        type="text"
        value={repository}
        onChange={(e) => setRepository(e.target.value)}
        placeholder="repository"
        aria-label="Agent repository"
        className={inputClass}
      />
      <input
        type="text"
        value={commit}
        onChange={(e) => setCommit(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
        }}
        placeholder="commit (optional)"
        aria-label="Agent commit"
        className={inputClass}
      />
      <button
        type="button"
        onClick={submit}
        disabled={!canSubmit}
        className={cn(
          "px-3",
          "py-1.5",
          "rounded",
          "text-sm",
          "text-white",
          canSubmit
            ? cn("bg-blue-600", "hover:bg-blue-700", "cursor-pointer")
            : cn("bg-neutral-400", "dark:bg-neutral-700", "cursor-not-allowed"),
        )}
      >
        Open
      </button>
    </div>
  );
}

interface ActiveTabBodyProps {
  tab: PanelTab;
  setPanelTabs: Dispatch<SetStateAction<PanelTab[]>>;
  onDraftChange: (value: string) => void;
  onAttachmentsChange: (
    next: AgentCompletionsMessageRichContentPart[],
  ) => void;
  onSend: () => void;
}

function ActiveTabBody({
  tab,
  setPanelTabs,
  onDraftChange,
  onAttachmentsChange,
  onSend,
}: ActiveTabBodyProps) {
  return (
    <div className={cn("flex", "flex-col", "flex-1", "min-h-0")}>
      <div
        className={cn(
          "flex-1",
          "min-h-0",
          "overflow-y-auto",
          "bg-neutral-50",
          "dark:bg-neutral-950",
        )}
      >
        <MessageList tab={tab} setPanelTabs={setPanelTabs} />
      </div>

      <ChatPane
        value={tab.draft}
        onChange={onDraftChange}
        attachments={tab.attachments}
        onAttachmentsChange={onAttachmentsChange}
        onSend={onSend}
      />
    </div>
  );
}
