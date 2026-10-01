// The page's only door to Rust: one typed wrapper per registry action.
// Types come from src/bindings (generated from Rust — never edit them).

import type { Reach } from "../bindings/Reach";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ActionInfo } from "../bindings/ActionInfo";
import type { AgentsListed } from "../bindings/AgentsListed";
import type { AppInfo } from "../bindings/AppInfo";
import type { FileNoticeView } from "../bindings/FileNoticeView";
import type { KeysBrokenView } from "../bindings/KeysBrokenView";
import type { CreateAgentInput } from "../bindings/CreateAgentInput";
import type { CreateOutcome } from "../bindings/CreateOutcome";
import type { DeleteOutcome } from "../bindings/DeleteOutcome";
import type { CallOutcome } from "../bindings/CallOutcome";
import type { CardEvent } from "../bindings/CardEvent";
import type { ToolView } from "../bindings/ToolView";
import type { Capacity } from "../bindings/Capacity";
import type { FeedRead } from "../bindings/FeedRead";
import type { HomeMove } from "../bindings/HomeMove";
import type { HostOutcome } from "../bindings/HostOutcome";
import type { HostSpaceInput } from "../bindings/HostSpaceInput";
import type { InviteView } from "../bindings/InviteView";
import type { JoinOutcome } from "../bindings/JoinOutcome";
import type { KnockEvent } from "../bindings/KnockEvent";
import type { SpaceEvent } from "../bindings/SpaceEvent";
import type { SpaceSummary } from "../bindings/SpaceSummary";
import type { SpaceView } from "../bindings/SpaceView";
import type { FileRead } from "../bindings/FileRead";
import type { FileWritten } from "../bindings/FileWritten";
import type { ImageKindView } from "../bindings/ImageKindView";
import type { LogEvent } from "../bindings/LogEvent";
import type { LogsQuery } from "../bindings/LogsQuery";
import type { MachineView } from "../bindings/MachineView";
import type { MessageOutcome } from "../bindings/MessageOutcome";
import type { NewMachineInput } from "../bindings/NewMachineInput";
import type { PersonView } from "../bindings/PersonView";
import type { ProfileView } from "../bindings/ProfileView";
import type { ProviderView } from "../bindings/ProviderView";
import type { SavedView } from "../bindings/SavedView";
import type { TabKind } from "../bindings/TabKind";
import type { TabsSnapshot } from "../bindings/TabsSnapshot";
import type { VolumeChange } from "../bindings/VolumeChange";
import type { VolumeMode } from "../bindings/VolumeMode";
import type { DoorView } from "../bindings/DoorView";
import type { AskSent } from "../bindings/AskSent";
import type { DoorwayView } from "../bindings/DoorwayView";
import type { AdmittedView } from "../bindings/AdmittedView";
import type { AppearAs } from "../bindings/AppearAs";
import type { PersonaView } from "../bindings/PersonaView";
import type { AllowanceView } from "../bindings/AllowanceView";
import type { EditMountsInput } from "../bindings/EditMountsInput";
import type { EditOutcome } from "../bindings/EditOutcome";
import type { MountsView } from "../bindings/MountsView";
import type { VolumeStat } from "../bindings/VolumeStat";
import type { VolumeTree } from "../bindings/VolumeTree";
import type { VolumesListed } from "../bindings/VolumesListed";

function channel<T>(onEvent: (event: T) => void): Channel<T> {
  const ch = new Channel<T>();
  ch.onmessage = onEvent;
  return ch;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  identityBroken: () => invoke<KeysBrokenView | null>("identity_broken"),
  filesSetAside: () => invoke<FileNoticeView[]>("files_set_aside"),
  asksClose: (thread: string, note: string | null) => invoke<AskSent[]>("asks_close", { thread, note }),
  actions: () => invoke<ActionInfo[]>("actions_list"),
  catalog: () => invoke<ImageKindView[]>("catalog_images"),
  check: (kind: string, args: unknown) => invoke<null>("catalog_check", { kind, arguments: args }),

  agentsList: () => invoke<AgentsListed>("agents_list"),
  agentsCreate: (input: CreateAgentInput) => invoke<CreateOutcome>("agents_create", { input }),
  agentsDelete: (name: string) => invoke<DeleteOutcome>("agents_delete", { name }),
  agentsMessage: (name: string, text: string, ticket: string) =>
    invoke<MessageOutcome>("agents_message", { name, text, ticket }),
  agentsEdit: (input: EditMountsInput) => invoke<EditOutcome>("agents_edit", { input }),
  agentsMounts: (name: string) => invoke<MountsView | null>("agents_mounts", { name }),
  takeBack: (ticket: string) => invoke<boolean>("agents_message_take_back", { ticket }),

  logsOpen: (query: LogsQuery, onEvent: (e: LogEvent) => void) =>
    invoke<string>("logs_open", { query, onEvent: channel(onEvent) }),
  scopeClose: (id: string) => invoke<boolean>("scope_close", { id }),

  // Every volume verb is addressed to one machine: a volume is its own.
  volumes: (machine: ProviderView) => invoke<VolumesListed>("volumes_list", { machine }),
  volumeStat: (machine: ProviderView, name: string) => invoke<VolumeStat>("volumes_stat", { machine, name }),
  volumeTree: (machine: ProviderView, name: string) => invoke<VolumeTree>("volumes_tree", { machine, name }),
  volumeRead: (machine: ProviderView, name: string, path: string) => invoke<FileRead>("volumes_read", { machine, name, path }),
  volumeWrite: (machine: ProviderView, name: string, path: string, text: string) => invoke<FileWritten>("volumes_write", { machine, name, path, text }),
  volumeRoom: (machine: ProviderView) => invoke<Capacity>("volumes_room", { machine }),
  volumeCreate: (machine: ProviderView, name: string, bytes: number, mode: VolumeMode) => invoke<VolumeChange>("volumes_create", { machine, name, bytes, mode }),
  volumeRoomFor: (machine: ProviderView, name: string) => invoke<Capacity>("volumes_room_for", { machine, name }),
  volumeEdit: (machine: ProviderView, name: string, bytes: number | null, mode: VolumeMode | null) => invoke<VolumeChange>("volumes_edit", { machine, name, bytes, mode }),
  volumeDelete: (machine: ProviderView, name: string) => invoke<VolumeChange>("volumes_delete", { machine, name }),

  homeFeed: () => invoke<HomeMove[]>("home_feed"),
  people: () => invoke<PersonView[]>("people_list"),
  profile: () => invoke<ProfileView>("profile_get"),
  spacesHome: () => invoke<string | null>("spaces_home"),
  spaces: () => invoke<SpaceSummary[]>("spaces_list"),
  space: (id: string) => invoke<SpaceView>("spaces_get", { id }),
  spaceFeed: (id: string) => invoke<FeedRead>("spaces_feed", { id }),
  spaceCall: (id: string, tool: string, args: unknown) => invoke<CallOutcome>("spaces_call", { id, tool, arguments: args }),
  spaceWatch: (id: string, onEvent: (e: SpaceEvent) => void) => invoke<string>("spaces_watch", { id, onEvent: channel(onEvent) }),
  spaceHost: (input: HostSpaceInput) => invoke<HostOutcome>("spaces_host", { input }),
  spaceDoor: (invite: string) => invoke<DoorView>("spaces_door", { invite }),
  spaceJoin: (invite: string, appearAs: AppearAs, note: string, listed: boolean, vouch: string | null) => invoke<JoinOutcome>("spaces_join", { invite, appearAs, note, listed, vouch }),
  spacesDoorways: (id: string) => invoke<DoorwayView[]>("spaces_doorways", { id }),
  vouchFor: (key: string, name: string, room: string) => invoke<string>("vouch_for", { key, name, room }),
  spacesAdmitted: (id: string) => invoke<AdmittedView[]>("spaces_admitted", { id }),
  spacesRestart: (id: string) => invoke<null>("spaces_restart", { id }),
  spacesContinue: (id: string) => invoke<HostOutcome>("spaces_continue", { id }),
  asksSend: (what: string, needs: string | null, ceiling: string | null, rooms: string[]) => invoke<AskSent[]>("asks_send", { what, needs, ceiling, rooms }),
  tableTree: (id: string) => invoke<VolumeTree>("table_tree", { id }),
  tableRead: (id: string, path: string) => invoke<FileRead>("table_read", { id, path }),
  tableWrite: (id: string, path: string, text: string) => invoke<FileWritten>("table_write", { id, path, text }),
  tableTransfer: (from: string, path: string, to: string) => invoke<FileWritten>("table_transfer", { from, path, to }),
  personas: () => invoke<PersonaView[]>("personas_list"),
  personaRename: (id: string, name: string) => invoke<null>("persona_rename", { id, name }),
  allowanceGet: (id: string, agent: string) => invoke<AllowanceView>("allowance_get", { id, agent }),
  allowanceSet: (id: string, agent: string, reach: Reach, perDay: number) => invoke<AllowanceView>("allowance_set", { id, agent, reach, perDay }),
  spaceLeave: (id: string) => invoke<null>("spaces_leave", { id }),
  spaceInvite: (id: string) => invoke<InviteView | null>("spaces_invite", { id }),
  knocksWatch: (onEvent: (e: KnockEvent) => void) => invoke<string>("knocks_watch", { onEvent: channel(onEvent) }),
  knocksAnswer: (knockId: number, yes: boolean) => invoke<null>("knocks_answer", { knockId, yes }),
  cardsWatch: (onEvent: (e: CardEvent) => void) => invoke<string>("cards_watch", { onEvent: channel(onEvent) }),
  cardsAnswer: (id: number, answer: string) => invoke<null>("cards_answer", { id, answer }),
  doorTools: () => invoke<ToolView[]>("door_tools"),

  machines: () => invoke<MachineView[]>("machines_list"),
  machineAdd: (input: NewMachineInput) => invoke<MachineView>("machines_add", { input }),
  machineRemove: (identity: ProviderView) => invoke<null>("machines_remove", { identity }),
  machineNames: () => invoke<Record<string, string>>("machines_names"),
  machineRename: (identity: ProviderView, name: string) => invoke<Record<string, string>>("machines_rename", { identity, name }),
  onMenu: (handler: (id: string) => void) => listen<string>("menu://action", (e) => handler(e.payload)),

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
