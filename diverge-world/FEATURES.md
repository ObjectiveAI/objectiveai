# Diverge World — features

Every feature the world has, in plain words. This file is kept current:
a change that adds, removes or alters a feature updates it in the same
commit. Where a behaviour is a rule in the code, the file it lives in
is named, so the two can be checked against each other.

## At a glance

- A dark grid with every account, agent, tool and volume on it as a
  glowing node, and lines for how they are connected.
- Nodes light up while they work, and pulses of light run along the
  lines as events happen.
- A panel at the top left shows and hides each kind of node and each
  type of line, with counts, and can arrange the board.
- Clicking a node opens a sheet along the bottom: its **logs**, its
  **filesystem**, and a **console** into its container.
- Files can be downloaded (right-click) and uploaded (drag and drop),
  however large.

---

## The board

`scenes/board.ts`, `src/grid/`

### What is on it

| Kind | Shape | Colour |
|---|---|---|
| Account | ◎ an open ring around a dot | yellow |
| Agent | ⬡ a hexagon inside a hexagon | cyan |
| Standalone tool | ◇ a diamond with a bar across | orange |
| Dependency tool | □ a small square | magenta |
| Volume | ⛁ a cylinder | green |

- **Who is shown.** Every account, agent, standalone tool, dependency
  tool and volume the signed-in account may list. A party that appears
  only in an event, with no list holding it, is drawn too, but cannot
  be opened.
- **Names.** Each node has its name beneath it. A tool is named by its
  image's `DIVERGE.md` `name` when the image gives one.
- **Starting places.** Nodes first stand in one column per kind
  (`grid/place.ts`).

### Live state (`grid/live.ts`)

- **Working:** an agent or tool that is active (its loop running, or a
  call or read in flight) has a breathing halo.
- **Connected:** an account that is connected has a steady halo.
- **Down:** an agent or tool whose container is not deployed stands
  faint.
- These update live, as the daemon's lists change. The nodes themselves
  do not; see [Limits](#limits).

### Lines (`grid/edges.ts`, `grid/styles.ts`, `grid/links.ts`)

| Type | Looks like | Means |
|---|---|---|
| Call | thin solid line in the sender's colour, chevron at the receiver | something sent to something else (an event ran that way) |
| Attachment | dashed grey line, no mark | a tool attached to an agent (or to its template), a dependency tool to the agents that depend on it, a volume to what mounts it |
| Creator | dotted line in the maker's colour, open dot at the maker | who made it: the account, agent or template that created an agent, tool or account |

- **Lanes.** Several lines between the same two nodes run side by side
  in lanes, in a fixed order, and keep that order however the nodes are
  dragged.
- **Hidden types.** Hiding a type of line closes its lanes up.

### Pulses (`grid/pulses.ts`)

- Every new event sends a dot of light along its call line, from sender
  to receiver, in the sender's colour.
- A burst of events sends at most a few dots at once, spaced out.
- Pulses follow a node while it is being dragged.

### Moving around (`scenes/camera.ts`)

- **Pan:** drag the empty board with any mouse button.
- **Zoom:** the mouse wheel, from a quarter size to three times, around
  the pointer.
- **Bounds:** the view's middle cannot leave the area the nodes cover,
  so you can always reach every node however far you zoom.
- **The grid** (`scenes/backdrop.ts`) is redrawn sharp at every zoom.
  It has a bright line every fifth square; the faint lines in between
  fade out as you zoom out rather than turning into a haze.

### Dragging nodes (`grid/nodes.ts`)

- Press a node and move it to drag it. Its halo, name, lines, selection
  ring and pulses all go with it.
- Positions last only for the page. A reload puts every node back in
  its starting column.

### Hover focus

Pointing at a node focuses it:
- its lines stay bright, and every other line goes faint;
- every node it has no shown line to dims;
- only its direct connections stay lit.

When nothing is hovered, the node whose sheet is open stays focused the
same way.

---

## The top-left panel

`scenes/ui.ts`

### Status line

- It shows only when there is something to say: "waiting for the
  daemon" while the world cannot reach it, or which lists this account
  may not read.
- Otherwise it is hidden.

### Legend

One line per kind of node, then one per type of line. Each shows its
key, its sample shape or stroke, its name, and how many the board
holds.

| Key | Toggles |
|---|---|
| `1` | accounts |
| `2` | agents |
| `3` | standalone tools |
| `4` | dependency tools |
| `5` | volumes |
| `6` | calls |
| `7` | attachments |
| `8` | creators |

- Click an entry, or press its key, to hide or show it. A hidden entry
  is drawn faint.
- **Hiding a kind:** the nodes go, and their lines and pulses with them.
  An open sheet for a node that is hidden closes.
- **Hiding a line type:** its lines and lanes go.

### Arrange

- The **Arrange** button, or the `A` key, lays out every shown node by
  the shown lines (`grid/arrange.ts`).
- The nodes glide to their new places over about 0.7 s.
- **The goals:** fewer crossings, no line through a node it does not
  belong to, no overlapping nodes or names, no lines bunched at one
  node, and a board that stays compact.
- **Each press starts from where the nodes are now**, so pressing again
  can give a different, often better, layout.
- It does nothing while a node is being dragged or nodes are still
  gliding.

---

## The sheet

`src/sheet/`

### Opening and closing

- **Open:** click a node. It gets a white selection ring, and its sheet
  opens along the bottom of the window.
- **Swap:** click another node to switch the sheet to it.
- **Close:** click the same node again, or press **✕**.
- **Resize:** drag the handle at the top, from 20% to 90% of the window
  height.
- **The header:** the node's name, a second line of detail (its
  template and index, who owns a dependency tool, an account's issuer),
  and a green dot while an agent's run is up or a tool has a request in
  flight.

### Tabs by kind

| Kind | Logs | Filesystem | Console |
|---|---|---|---|
| Agent | yes | yes | yes |
| Standalone tool | yes | yes | yes |
| Dependency tool | yes | yes | yes |
| Account (and admin) | yes | — | — |
| Volume | — | yes | — |

- The tab bar shows only when a node has more than one tab.
- A tab starts reading the first time it is shown, and keeps running
  until the sheet closes, so switching back finds it current.
- The tab you last chose is remembered, and the next node opens on it
  if it has it.

---

## The logs tab

`sheet/log.ts`, `sheet/fold.ts`, `sheet/tool_fold.ts`

The log is read from the start and then followed live.

### An agent's log

- **Messages:** the conversation as messages. The assistant's text
  joins into one message per turn, and an image breaks it.
- **Text:** rendered as Markdown. Reasoning is folded away under its
  own heading.
- **Tool calls:** each is a card, closed until opened, holding its
  arguments and, once answered, its response.
- **Senders:** user parts are grouped per sender and labelled with who
  sent them (an account, another agent or a tool).
- **Errors:** errors and refusals are shown in red.

### A tool's log

- **Callers:** every call and resource read routed into the tool, as a
  card per request with its answer, grouped by caller.
- **Notifications:** shown under the caller whose session they arrived
  on.
- **Commands:** every request the tool's own container put to the
  daemon, marked allowed or refused.

### An account's log

Every request the account put to the daemon, marked allowed or
refused. An administrator's log is the daemon's own admin log.

### Scrolling

- The log opens at the bottom and stays there as new entries arrive.
- Scroll up and the view holds still while entries keep arriving
  below.
- Scroll back to the bottom and it follows again.
- While the history is still loading, the log cannot be scrolled, so
  it always lands at the bottom.

---

## The filesystem tab

`sheet/files.ts`, `sheet/filetree.ts`

### The tree

- **Columns:** name, size, created, modified. Directories come first at
  every level, shown as `name/`. Symbolic links show as
  `name → target`.
- **Sorting:** click a column header to sort by it ascending, again for
  descending, again to stop. Missing values show as `—` and sort last.
- **Folders:** click a directory to open or close it. What is open
  stays open through every change. An open empty directory says
  "empty".
- **Live changes:** a file or directory that changes glows copper
  briefly. A closed directory glows for changes inside it.
- **Possibly stale:** a part of a container the daemon cannot watch is
  dimmed, with a tooltip saying it may be stale.
- **Selection:** clicking or right-clicking a row selects it, like a
  file manager. One row at a time, highlighted more strongly than the
  hover light (a selected row does not change on hover). Clicking empty
  space clears the selection.

### Containers (agents, standalone tools, dependency tools)

- **Live:** the whole container's filesystem is watched, and updates
  live.
- **Starting it:** opening an agent's or tool's filesystem starts its
  container if it is stopped.
- **Keeping it up:** an open filesystem tab keeps the container running.
  The idle timer only starts once no file tree is open and no upload or
  download is in progress.
- **When it stops:** the tab says "the container stopped" and keeps the
  last tree.
- **Dependency tools** are never started for this. One that is not
  running says "not running".

### Volumes

- **A snapshot:** a volume's files are read once, not watched. **↻**
  in the header reads them again.
- **In use:** a volume mounted by a running container can't be read
  while it runs, and says "in use by a running container".

### Download

1. Right-click a row and choose **Download** (or **Download folder** on
   a directory).
2. The system dialog asks where to save. A folder is saved as a new
   folder of its own name, and is refused if one is already there.
3. The transfer shows in the transfers panel.

### Upload

- Drag files or folders from your desktop onto the tree.
- **Where they land:** the directory under the pointer, a file's own
  directory, or the root on empty space. The target directory and its
  open contents light up, and a banner says where they will land.
- Files of the same name are replaced.
- A volume's tree is read again once the upload lands. A container's
  updates by itself.

### The transfers panel (`sheet/strip.ts`)

- It stands at the bottom right of the window, above the sheet, and
  stays after the sheet closes.
- **Each transfer:** its direction and name, a progress bar when the
  total is known, and the bytes moved.
- **✕** stops a running transfer, or dismisses a finished one.
- **Results:** a finished transfer says so and leaves after a few
  seconds. A failed one says why and stays until dismissed.

### How transfers work (`src-tauri/src/transfer.rs`)

- The app's own Rust moves the bytes between your disk and the daemon,
  over the page's connection. They never pass through the page.
- **Memory stays flat, however large the file.** Files move in pieces
  of at most 2 MiB, and every hop holds at most one piece ahead (the
  wire's flow control).
- Stopping a transfer tells the daemon at once.

---

## The console tab

`sheet/console.ts`

- **A real terminal.** One interactive shell in the container (`bash`,
  or `sh` where there is no bash), drawn by xterm: colours, cursor
  movement and full-screen programs work. `cd`, variables and running
  programs carry on from one command to the next, because it is one
  shell.
- **Starting it:** the shell opens the first time the tab is shown,
  starting the container if it is stopped; it keeps the container
  running while it is open. Dependency tools are never started for it.
- **Typing:** keys and pastes go straight to the shell, in order, even
  while it is printing.
- **Size:** the terminal fills the sheet; resizing the sheet or the
  window tells the shell its new size.
- **Switching tabs** keeps the shell; closing the sheet or opening
  another node kills it.
- **When it ends:** the tab says how (`exited 0`, killed by a signal,
  not found, forbidden, the container stopped), and any key opens a new
  shell.
- **Permission:** the account needs the `exec` grant over the node.

---

## Behind the scenes

- **The daemon is read once.** The world reads the daemon's lists once
  when it opens (retrying every 5 seconds while it cannot), then
  follows live lights, pulses, logs and trees.
- **Reconnecting.** The connection is held by the app's Rust and redials
  by itself as `viewer.yaml` says.
- **Nothing is saved.** The app keeps no files of its own.

## Limits

These are the gaps as they stand:

- **New or removed nodes** appear only after a reload.
- **Node positions** are lost on reload.
- **Opening a file** in place is not possible: files can be browsed,
  downloaded and uploaded, but not viewed in the app.
- **One console per sheet.** A shell lives only while its node's sheet
  is open; there is no reattaching to an earlier one.
- **Desktop only.** Downloading and uploading need a desktop window.
