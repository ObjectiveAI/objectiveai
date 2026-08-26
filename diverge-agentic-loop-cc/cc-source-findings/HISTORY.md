# Claude Code conversation history — on-disk mechanics

Source: the unminified Claude Code source cloned at the workspace root as `temp/`
(github.com/codeaashu/claude-code). Every claim below cites `file:line` in that tree and was
either read directly or verified by spot-check; where the source does not say, this document
says **not found** rather than guessing. Auth is out of scope. Purpose: the `-cc` container is
stateless — the continuation token must carry everything needed to reconstruct these files at
run time.

---

## 1. Where everything lives

**Base dir** — `src/utils/envUtils.ts:7-14` (verified verbatim):

```ts
export const getClaudeConfigHomeDir = memoize(
  (): string => {
    return (
      process.env.CLAUDE_CONFIG_DIR ?? join(homedir(), '.claude')
    ).normalize('NFC')
  },
  () => process.env.CLAUDE_CONFIG_DIR,
)
```

So base `C` = `$CLAUDE_CONFIG_DIR` or `~/.claude`, NFC-normalized. This one function is the
single base-dir definition; `CLAUDE_CONFIG_DIR` relocates essentially the whole state tree.

**Projects dir** — `C/projects` (`src/utils/sessionStorage.ts:198-200`, duplicated for the
SDK in `src/utils/sessionStoragePortable.ts:325-327`).

**Per-project directory name** — `sanitizePath(cwd)`,
`src/utils/sessionStoragePortable.ts:311-319` (verified verbatim), `MAX_SANITIZED_LENGTH =
200` (`:293`):

```ts
export function sanitizePath(name: string): string {
  const sanitized = name.replace(/[^a-zA-Z0-9]/g, '-')
  if (sanitized.length <= MAX_SANITIZED_LENGTH) {
    return sanitized
  }
  const hash =
    typeof Bun !== 'undefined' ? Bun.hash(name).toString(36) : simpleHash(name)
  return `${sanitized.slice(0, MAX_SANITIZED_LENGTH)}-${hash}`
}
```

Every non-alphanumeric character of the absolute cwd becomes `-`; over 200 chars, truncate to
200 and append `-<base36 hash>` (with a documented Bun.hash-vs-simpleHash divergence between
CLI and SDK, `sessionStoragePortable.ts:347-353`, and a prefix-scan fallback `findProjectDir`
at `:354-380`).

**The cwd that feeds it is realpath'd and NFC-normalized** — `src/bootstrap/state.ts:260-279`:
`resolvedCwd = realpathSync(rawCwd).normalize('NFC')` (EPERM fallback: NFC only). Both
`originalCwd` and `projectRoot` get this value. `getProjectRoot` is "never updated by
mid-session EnterWorktreeTool" (`state.ts:504-510`). Consequence: mounting a project at a
different path changes the directory name and makes prior sessions invisible.

**Transcript path** — `src/utils/sessionStorage.ts:202-205` (verified verbatim):

```ts
export function getTranscriptPath(): string {
  const projectDir = getSessionProjectDir() ?? getProjectDir(getOriginalCwd())
  return join(projectDir, `${getSessionId()}.jsonl`)
}
```

`sessionProjectDir` is an in-memory override set by `switchSession(sessionId, projectDir)`
for cross-project/worktree resume (`state.ts:461-479`); `null` means derive from cwd.
`bootstrap/state.ts` itself touches no files — it is a pure in-memory singleton (only fs
import is `realpathSync`, `state.ts:7`).

**Session id** = v4 UUID: `sessionId: randomUUID()` (`state.ts:331`, regenerated at `:447`).
On disk, only `.jsonl` files whose basename passes a UUID validation are treated as sessions
(`sessionStorage.ts:4546-4549`; `listSessionsImpl.ts:183-185`).

**Permissions**: directories `0o700`, files `0o600` throughout (`sessionStorage.ts:634-643`,
`:1601`, `:1608`, `:2579-2582`).

---

## 2. The transcript file: write mechanics

- **Append-only JSONL**: every line is `jsonStringify(entry) + '\n'` (`sessionStorage.ts:656`,
  `:2577`). Core append (`:634-643`): `appendFile(path, data, {mode:0o600})`, retrying after
  `mkdir(dirname, {recursive:true, mode:0o700})` on any failure.
- **Batching**: writes go through a per-file queue drained on a `setTimeout` of
  `FLUSH_INTERVAL_MS = 100` (`:567`, 10ms when a remote writer is registered, `:530`), chunks
  bounded by 100 MiB (`:568`). A sync path `appendEntryToFile` (`:2571-2584`,
  `appendFileSync`) is used by metadata saves and exit cleanup.
- **No fsync, no locks, no temp-file rename** anywhere in the module (verified by the deep
  read).
- **Lazy creation**: nothing is written until the first `user`/`assistant` message arrives
  (`:1005-1010` → `materializeSessionFile`, `:976-991`); earlier entries buffer in memory.
- **Exit flush**: `registerCleanup` flushes the queue and re-appends session metadata
  (`:449-462`).
- **Suppression** (`shouldSkipPersistence`, `:960-970`): `NODE_ENV=test` (unless
  `TEST_ENABLE_SESSION_PERSISTENCE`), settings `cleanupPeriodDays: 0`,
  `setSessionPersistenceDisabled(...)`, or `CLAUDE_CODE_SKIP_PROMPT_HISTORY`.
- **Dedup**: message UUIDs already in the in-memory set are not re-appended (`:1242-1262`);
  metadata entry types bypass dedup.
- **The only non-append writes** (`:871-951`, `:1587-1622`, `:1632-1723`): UUID tombstone
  removal (tail-truncate or full rewrite, skipped above 50 MiB), and two remote-hydration
  paths that overwrite the whole file with server-provided content.

---

## 3. Record shapes (one JSON object per line)

The line union is `Entry` — `src/types/logs.ts:297-317` (verified): `TranscriptMessage`,
`SummaryMessage`, `CustomTitleMessage`, `AiTitleMessage`, `LastPromptMessage`,
`TaskSummaryMessage`, `TagMessage`, `AgentNameMessage`, `AgentColorMessage`,
`AgentSettingMessage`, `PRLinkMessage`, `FileHistorySnapshotMessage`,
`AttributionSnapshotMessage`, `QueueOperationMessage`, `SpeculationAcceptMessage`,
`ModeEntry`, `WorktreeStateEntry`, `ContentReplacementEntry`, `ContextCollapseCommitEntry`,
`ContextCollapseSnapshotEntry`.

### 3a. Conversation lines

The single write-site literal, `insertMessageChain` (`sessionStorage.ts:1039-1064`):

```ts
const transcriptMessage: TranscriptMessage = {
  parentUuid: isCompactBoundary ? null : effectiveParentUuid,
  logicalParentUuid: isCompactBoundary ? parentUuid : undefined,
  isSidechain,
  teamName: teamInfo?.teamName,
  agentName: teamInfo?.agentName,
  promptId: message.type === 'user' ? (getPromptId() ?? undefined) : undefined,
  agentId,
  ...message,
  userType: getUserType(),      // process.env.USER_TYPE || 'external' (:419-421)
  entrypoint: getEntrypoint(),  // process.env.CLAUDE_CODE_ENTRYPOINT (:423-425)
  cwd: getCwd(),
  sessionId,
  version: VERSION,
  gitBranch,                    // `git branch` once per chain; undefined on failure (:1013-1019)
  slug,                         // plan slug cache (:1022-1023)
}
```

`parentUuid` is deliberately the FIRST key — byte-level scanners depend on the
`{"parentUuid":` prefix (`:3245-3250`, `:3310`). For tool results, the parent is overridden
to `sourceToolAssistantUUID` (`:1030-1037`).

Wrapper types (`src/types/logs.ts:221-231`, `:8-17`):

```ts
TranscriptMessage = SerializedMessage & {
  parentUuid: UUID | null, logicalParentUuid?, isSidechain: boolean,
  gitBranch?, agentId?, teamName?, agentName?, agentColor?, promptId? }
SerializedMessage = Message & {
  cwd, userType, entrypoint?, sessionId, timestamp, version, gitBranch?, slug? }
```

The `Message` union (`src/types/message.ts:336-343`, verified): `AssistantMessage |
UserMessage | SystemMessage | AttachmentMessage | ProgressMessage | TombstoneMessage`.
Persisted `type` values are `'user' | 'assistant' | 'attachment' | 'system'`
(`sessionStorage.ts:139-146`); `progress` is explicitly not persisted (`:128-146`, `:4352`),
though legacy on-disk progress lines are tolerated (`:158-178`).

Per-type fields (verified in `src/types/message.ts`):

- **UserMessage** (`:95-121`): `type:'user'`, `message: { role:'user', content: string |
  ContentBlockParam[] }`, `uuid`, `timestamp`, and optionals `isMeta`,
  `isVisibleInTranscriptOnly`, `isVirtual`, `isCompactSummary`, `toolUseResult`, `mcpMeta`,
  `imagePasteIds`, `sourceToolAssistantUUID`, `permissionMode`, `summarizeMetadata`,
  `origin`.
- **AssistantMessage** (`:72-89`): `type:'assistant'`, `uuid`, `timestamp`, `message:
  BetaMessage` (the Anthropic API message — carries `message.id` and `message.usage.*`,
  consumed at `sessionStorage.ts:1926-1938`), optionals `requestId`, `isMeta`, `isVirtual`,
  `isApiErrorMessage`, `apiError`, `error`, `errorDetails`, `advisorModel`, `agentId`,
  `caller`.
- **SystemMessage** (`:264-279`): base `{type:'system', uuid, timestamp, isMeta?, content?,
  level?, toolUseID?}` plus a `subtype` union of sixteen kinds, including
  `turn_duration` (carries `messageCount`, `sessionStorage.ts:2227-2229`) and the compact /
  microcompact boundary messages.
- **AttachmentMessage** (`:285-292`): `type:'attachment'`, `attachment: {type: string, ...}`,
  `uuid`, `timestamp`, `isMeta?`. Filtered on load for non-`ant` users except hook context
  (`sessionStorage.ts:4351-4367`).

### 3b. Session-metadata lines

Written by `reAppendSessionMetadata` (`sessionStorage.ts:721-839`) and the `save*` helpers
(sync appends), and **re-appended at end of file after compaction/resume** so they stay
within the 64 KiB tail window the pickers read (`:693-720`, `:2807-2817`):

| `type` | shape | cite |
|---|---|---|
| `last-prompt` | `{type, lastPrompt, sessionId}` | :768-772 |
| `custom-title` | `{type, customTitle, sessionId}` | :777-781 |
| `ai-title` | `{type, aiTitle, sessionId}` | :2668-2672 |
| `task-summary` | `{type, summary, sessionId, timestamp}` | :2682-2687 |
| `tag` | `{type, tag, sessionId}` | :784-788 |
| `agent-name` / `agent-color` / `agent-setting` | `{type, agentName/agentColor/agentSetting, sessionId}` | :791-809 |
| `mode` | `{type, mode: 'coordinator'\|'normal', sessionId}` | :811-817 |
| `worktree-state` | `{type, worktreeSession, sessionId}` (stripped to `PersistedWorktreeSession`, `null` = exited) | :818-824, :2895-2907 |
| `pr-link` | `{type, sessionId, prNumber, prUrl, prRepository, timestamp}` | :830-838 |

### 3c. Other line kinds

- `summary`: `{type:'summary', leafUuid, summary}` (`logs.ts:55-59`) — read and appendable in
  sessionStorage, but its producer is elsewhere (not found in that file).
- `file-history-snapshot`: `{type, messageId, snapshot, isSnapshotUpdate}` (`:1091-1096`) —
  the checkpoint *index*; backup contents live in `C/file-history/` (§6).
- `content-replacement`: `{type, sessionId, agentId?, replacements}` (`:1118-1123`) — records
  large tool results offloaded to `tool-results/` files.
- `queue-operation`, `attribution-snapshot`: passed through from callers (`:1101-1111`).
- `marble-origami-commit` / `marble-origami-snapshot` (context collapse): `{type, sessionId,
  collapseId, summaryUuid, summaryContent, summary, firstArchivedUuid, lastArchivedUuid}` /
  `{type, sessionId, staged[...], armed, lastSpawnTokens}` (`:1542-1580`).
- **Compaction**: the boundary is a system message; on disk it severs the chain
  (`parentUuid: null`) and keeps the logical link in `logicalParentUuid` (`:1026`,
  `:1040-1041`). `compactMetadata.preservedSegment {headUuid, tailUuid, anchorUuid}` and
  `snipMetadata.removedUuids` drive relinking/pruning on load (`:1839-1956`, `:1982-2039`).
  Files over 5 MiB skip pre-boundary bytes on load and recover only metadata lines from the
  prefix (`:3536-3556`, marker list at `:3113-3123`;
  kill switch `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP`).

---

## 4. Sidechains / subagents

- `isSidechain` is stamped per line; a sidechain **with an `agentId`** routes to a separate
  file (`sessionStorage.ts:1224-1228`):
  `P/<sessionId>/subagents[/<subdir>]/agent-<agentId>.jsonl` (`:247-258`; the optional
  grouping subdir comes from an in-memory map, e.g. `subagents/workflows/<runId>/`).
  A sidechain without `agentId` stays in the main file.
- Sidecar metadata: `agent-<agentId>.meta.json` = `{agentType, worktreePath?, description?}`
  (`:260-272`, write `:283-290`).
- `remote-agents/remote-agent-<taskId>.meta.json` beside it (`:320-329`).
- Sidechain writes bypass UUID dedup and remote persistence (`:1230-1261`).
- Read-back: filter `agentId` + `isSidechain`, latest leaf, chain walk, then strip
  `isSidechain`/`parentUuid` (`:4190-4236`); discovery globs `agent-*.jsonl` (`:4328-4346`).
- The session pickers exclude sidechain sessions — first line containing
  `"isSidechain":true` disqualifies a file (`listSessionsImpl.ts:86-94`;
  `sessionStorage.ts:5055-5067`).

---

## 5. Reading back: resume

**Core reader** `loadTranscriptFile` (`sessionStorage.ts:3472-3813`): read file → per-line
`JSON.parse` with malformed lines silently skipped (`src/utils/json.ts:146-172`) → dispatch
by `type` into the message map + side maps (summaries, titles, tags, modes, worktree states,
snapshots, collapse records; `:3590-3610`) → apply preserved-segment relinks and snip
removals → compute `leafUuids` = "uuids that no other message's parentUuid points at"
(`conversationRecovery.ts:405-415`; computation `:3707-3790`).

**Chain reconstruction** `buildConversationChain` (`:2069-2094`): walk `parentUuid` from the
newest non-sidechain leaf to the root (cycle-guarded), reverse, then
`recoverOrphanedParallelToolResults` (`:2096-2206`) re-attaches sibling assistant blocks
(same `message.id`) and their orphaned `tool_result` user messages that the single-parent
walk dropped, ordered by timestamp.

**What callers receive**: `removeExtraFields` strips exactly `isSidechain` and `parentUuid`
(`:1814-1821`) — everything else on the line rides back into the session.

**Resume entry** `loadConversationForResume` (`conversationRecovery.ts:456-597`): source
branches for `--continue` (most recent log, skipping live background sessions), a raw
`.jsonl` path (cross-directory), or a session id. Post-load processing (`:164-252`): legacy
attachment migration, unknown `permissionMode` stripping, three filters (unresolved
tool_uses, orphaned thinking-only messages, whitespace-only assistant messages), interrupt
detection (`:272-333`, trailing tool_result or attachment ⇒ `interrupted_turn`, synthetic
`"Continue from where you left off."` meta user message appended `:210-224`), and a
`NO_RESPONSE_REQUESTED` sentinel after a trailing user message (`:226-245`).

**Consistency**: the last `system/turn_duration` checkpoint's `messageCount` is compared
against its chain index and logged on drift (`sessionStorage.ts:2224-2243`).

**Enumeration** (`listSessionsImpl.ts`): scans `C/projects/*/`, accepts only
`<uuid>.jsonl`, and reads ONLY the first and last 64 KiB of each file
(`LITE_READ_BUF_SIZE = 65536`, `sessionStoragePortable.ts:17`, `:256-282`), scraping fields
by substring (`extractJsonStringField`) rather than JSON parsing: title
(`customTitle` > `aiTitle`), first prompt (from head, skipping tool_results/meta/compact
summaries), `createdAt` from the first line's `timestamp`, summary precedence
`customTitle || lastPrompt || summary || firstPrompt`, `gitBranch`, `cwd`, `tag` (only from
a line starting `{"type":"tag"`). No message counts are computed. Worktree-aware scanning
matches sanitized prefixes across project dirs (`:309-401`).

---

## 6. Conversation-adjacent files (complete inventory found)

With `C` = config home, `P` = `C/projects/<sanitized-cwd>`, `S` = sessionId:

**Inside `P/`:**

| path | contents | cite |
|---|---|---|
| `P/<S>.jsonl` | the conversation | sessionStorage.ts:202-205 |
| `P/<S>-<timestamp>.cast` | asciinema recording (ant + `CLAUDE_CODE_TERMINAL_RECORDING`) | asciicast.ts:36-44 |
| `P/bridge-pointer.json` | `{sessionId, environmentId, source}` crash-recovery pointer | bridge/bridgePointer.ts:52-54 |
| `P/<S>/subagents/**` | subagent transcripts + `.meta.json` | sessionStorage.ts:247-272 |
| `P/<S>/remote-agents/*.meta.json` | remote agent metadata | sessionStorage.ts:320-329 |
| `P/<S>/tool-results/<toolUseId>.{json,txt}` | offloaded large tool results (message keeps a 2000-byte preview) | toolResultStorage.ts:27, :97-116 |
| `P/<S>/session-memory/summary.md` | session memory summary | permissions/filesystem.ts:261-271 |

**Under `C/` directly:**

| path | contents | cite |
|---|---|---|
| `C/history.jsonl` | global PROMPT history (up-arrow), not conversations: `{display, pastedContents, timestamp, project, sessionId}` lines, `0o600`, lockfile-guarded appends | history.ts:115, :219-225, :292-327 |
| `C/file-history/<S>/<sha256(path)[..16]>@v<N>` | pre-edit file backups (checkpoint contents) | fileHistory.ts:725-741 |
| `C/shell-snapshots/snapshot-<shell>-<ts>-<id>.sh` | captured shell env for Bash tool | bash/ShellSnapshot.ts:439-444 |
| `C/plans/<planSlug>.md`, `<planSlug>-agent-<id>.md` | plan-mode documents (slug is stamped in the transcript's `slug` field) | plans.ts:118-127 |
| `C/tasks/<taskListId>/<taskId>.json` + `.lock` | file-backed todo v2 (taskListId falls back to sessionId; NOTE: different sanitizer `[^a-zA-Z0-9_-]` → `-`) | tasks.ts:199-231, :505 |
| `C/teams/<name>/config.json` | team config | tasks.ts:727-728; envUtils.ts:16-18 |
| `C/sessions/<pid>.json` | live-session PID registry | concurrentSessions.ts:21-22, :64 |
| `C/image-cache/<S>/<imageId>.<ext>` | pasted/attached images | imageStore.ts:9, :17-35 |
| `C/paste-cache/<sha256(content)[..16]>.txt` | large pasted text bodies | pasteStore.ts:8, :12-30 |
| `C/uploads/<S>/<file>` | inbound bridge attachments | bridge/inboundAttachments.ts:60-62 |
| `C/debug/<S>.txt`, `C/debug/latest` | per-session debug log | debug.ts:230-240 |
| `C/CLAUDE.md`, `C/rules/` | user memory/rules injected into conversations | config.ts:1784, :1805-1807 |
| auto-memory: `<memory base>/projects/<sanitized-git-root>/memory/**` | MEMORY.md + topics + logs; keyed by canonical git root, shared across worktrees | memdir/paths.ts:85-93, :203, :223-233 |

**`~/.claude.json`** (note: `CLAUDE_CONFIG_DIR || homedir()` directly — no `.claude`
subdir fallback; `env.ts:13-25`): `projects[<abs path>].lastSessionId` plus last-cost/token
fields (`config.ts:75-108`). Prompt history was migrated OUT of it into `history.jsonl`
(`config.ts:963-988`).

**Not found anywhere in this tree** (searched): a `todos/` directory (SDK-path todos are
reconstructed by re-reading the transcript, `sessionRestore.ts:138-149`), a `summaries/`
directory, a standalone queued-messages file (queue ops are transcript lines), or
path-construction for `C/server-sessions.json` (comment only, `server/types.ts:43`).

---

## 7. Aging

`cleanupOldMessageFilesInBackground` (`cleanup.ts:575-602`), cutoff = `cleanupPeriodDays`
setting, default 30 days (`:23-31`); skipped entirely if settings are invalid while
containing that key (`:575-585`). Transcripts: every `.jsonl`/`.cast` under
`C/projects/*/` older than cutoff **by file mtime** is unlinked, session subdirectories
(`tool-results` etc.) cleaned and rmdir'd (`:155-258`). Also aged: plans, `file-history/`,
`session-env/`, `debug/` (keeping `latest`), image caches, pastes, stale worktrees.
`history.jsonl` is NOT in the cleanup set.

---

## 8. Environment variables that matter here

| var | effect | cite |
|---|---|---|
| `CLAUDE_CONFIG_DIR` | relocates the whole tree (and `~/.claude.json`) | envUtils.ts:10; env.ts:24 |
| `CLAUDE_CODE_SKIP_PROMPT_HISTORY` | disables ALL transcript persistence (and prompt history) | sessionStorage.ts:968; history.ts:414-416 |
| settings `cleanupPeriodDays: 0` | disables transcript persistence | sessionStorage.ts:966 |
| `CLAUDE_CODE_ENTRYPOINT` | stamped on every line as `entrypoint` | sessionStorage.ts:423-425 |
| `USER_TYPE` | stamped as `userType` (default `'external'`); gates attachment persistence and `.cast` | sessionStorage.ts:419-421, :4351-4367 |
| `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP` | disables the >5 MiB pre-boundary skip on load | sessionStorage.ts:3536 |
| `CLAUDE_CODE_TASK_LIST_ID` / `CLAUDE_CODE_TEAM_NAME` | select the `C/tasks/<id>/` dir | tasks.ts:195-209 |
| `CLAUDE_CODE_REMOTE_MEMORY_DIR`, `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` | relocate auto-memory | memdir/paths.ts:85-88, :212 |

---

## 9. Derived: minimal reconstruction set for the stateless container

(Everything below follows from the citations above; nothing new is asserted.)

To make a session resumable at cwd `X` inside the container, with `C` chosen (either
`~/.claude` or via `CLAUDE_CONFIG_DIR`) and `D = sanitizePath(realpath(X))`:

1. `C/projects/D/<S>.jsonl` — the conversation itself; the chain is rebuilt from
   `parentUuid` links, so line order matters less than link integrity, but metadata records
   must sit near EOF to be seen by tail-window readers.
2. `C/projects/D/<S>/subagents/**` (+ `.meta.json`) — if subagent state matters.
3. `C/projects/D/<S>/tool-results/**` — if any `content-replacement` lines reference
   offloaded results.
4. `C/file-history/<S>/**` — only for rewind/undo of file edits.
5. `C/plans/<slug>*.md` — if the session used plan mode (`slug` field names it).
6. `C/tasks/<S or team>/**` — for todo-v2 state.
7. `C/image-cache/<S>/**`, `C/paste-cache/**`, `C/uploads/<S>/**` — for referenced media.
8. `~/.claude.json` `projects[X].lastSessionId` — what `--continue`-style flows read.
9. The container's project mount path MUST equal the recorded `cwd` (realpath'd,
   NFC-normalized), or `D` will not match and every session is invisible.
10. Writes are plain appends with no locking — reconstructing files wholesale before the
    process starts is safe; there is no index or cache to invalidate (in-memory memos only).
