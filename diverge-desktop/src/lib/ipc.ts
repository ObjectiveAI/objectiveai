// The page's only door to Rust: one typed wrapper per registry action.
// Types come from src/bindings (generated from Rust — never edit them).

import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ActionInfo } from "../bindings/ActionInfo";
import type { AgentsListed } from "../bindings/AgentsListed";
import type { AppInfo } from "../bindings/AppInfo";
import type { CreateAgentInput } from "../bindings/CreateAgentInput";
import type { CreateOutcome } from "../bindings/CreateOutcome";
import type { DeleteOutcome } from "../bindings/DeleteOutcome";
import type { FileRead } from "../bindings/FileRead";
import type { FileWritten } from "../bindings/FileWritten";
import type { ImageKindView } from "../bindings/ImageKindView";
import type { LogEvent } from "../bindings/LogEvent";
import type { LogsQuery } from "../bindings/LogsQuery";
import type { MachineView } from "../bindings/MachineView";
import type { MessageOutcome } from "../bindings/MessageOutcome";
import type { NewMachineInput } from "../bindings/NewMachineInput";
import type { ProviderView } from "../bindings/ProviderView";
import type { SavedView } from "../bindings/SavedView";
import type { TabKind } from "../bindings/TabKind";
import type { TabsSnapshot } from "../bindings/TabsSnapshot";
import type { TreeEvent } from "../bindings/TreeEvent";

function channel<T>(onEvent: (event: T) => void): Channel<T> {
  const ch = new Channel<T>();
  ch.onmessage = onEvent;
  return ch;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  actions: () => invoke<ActionInfo[]>("actions_list"),
  catalog: () => invoke<ImageKindView[]>("catalog_images"),
  check: (kind: string, args: unknown) => invoke<null>("catalog_check", { kind, arguments: args }),

  agentsList: () => invoke<AgentsListed>("agents_list"),
  agentsCreate: (input: CreateAgentInput) => invoke<CreateOutcome>("agents_create", { input }),
  agentsDelete: (name: string) => invoke<DeleteOutcome>("agents_delete", { name }),
  agentsMessage: (name: string, text: string, ticket: string) =>
    invoke<MessageOutcome>("agents_message", { name, text, ticket }),
  takeBack: (ticket: string) => invoke<boolean>("agents_message_take_back", { ticket }),

  logsOpen: (query: LogsQuery, onEvent: (e: LogEvent) => void) =>
    invoke<string>("logs_open", { query, onEvent: channel(onEvent) }),
  scopeClose: (id: string) => invoke<boolean>("scope_close", { id }),

  treeOpen: (path: string, onEvent: (e: TreeEvent) => void) =>
    invoke<string>("files_tree_open", { path, onEvent: channel(onEvent) }),
  fileRead: (path: string) => invoke<FileRead>("files_read", { path }),
  fileWrite: (path: string, text: string) => invoke<FileWritten>("files_write", { path, text }),

  machines: () => invoke<MachineView[]>("machines_list"),
  machineAdd: (input: NewMachineInput) => invoke<MachineView>("machines_add", { input }),
  machineRemove: (identity: ProviderView) => invoke<null>("machines_remove", { identity }),

  views: () => invoke<SavedView[]>("views_list"),
  viewSave: (view: SavedView) => invoke<SavedView>("views_save", { view }),
  viewDelete: (id: string) => invoke<null>("views_delete", { id }),

  tabs: () => invoke<TabsSnapshot>("tabs_snapshot"),
  tabOpen: (tab: TabKind) => invoke<TabsSnapshot>("tabs_open", { tab }),
  tabClose: (key: string) => invoke<TabsSnapshot>("tabs_close", { key }),
  tabFocus: (key: string) => invoke<TabsSnapshot>("tabs_focus", { key }),
  onTabs: (handler: (snapshot: TabsSnapshot) => void) =>
    listen<TabsSnapshot>("tabs://changed", (event) => handler(event.payload)),
};

export function ticket(): string {
  return `msg-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return JSON.stringify(error);
}
