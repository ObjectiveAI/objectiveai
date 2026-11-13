# Diverge World — features

Every feature the world has, in plain words. This file is kept current:
a change that adds, removes or alters a feature updates it in the same
commit. Where a behaviour is a rule in the code, the file it lives in
is named, so the two can be checked against each other.

## At a glance

- A dark grid with every account, agent, tool, volume, provider and daemon on it as a
  node, and lines for how they are connected — all of it live: what is
  made appears, what changes is redrawn, what is deleted goes. It is in
  the theme it shares with the viewer (`diverge-theme/theme.css`): the same five
  colours as every panel, nothing glowing.
- A line traces round a node while it works, and a short stroke runs
  along a line as each event happens.
- An outliner lists every node by family — clicking one flies the view to
  it and opens its sheet — shows and hides each kind and each type of
  line, with counts, and can arrange the board. It floats where you drag
  it, folds small, and docks down the window's left.
- Clicking a node opens a sheet along the bottom: its **logs**, its
  **filesystem**, a **console** into its container, and — for an agent
  or a tool that serves one — its **frontend**, its own web page.
- Files can be downloaded (right-click) and uploaded (drag and drop),
  however large.
- Another daemon our account reaches opens in a **tab** of its own,
  along the top of the window, from its daemon node's sheet — and
  everything in that tab works as at home.

---

## The board

`scenes/board.ts`, `src/grid/`

### What is on it

| Kind | Shape | Colour (its family, `grid/palette.ts`) |
|---|---|---|
| Account | ◎ an open ring around a dot | the accent — who acts |
| Agent | ⬡ a hexagon inside a hexagon | the accent — who acts |
| Standalone tool | ◇ a diamond with a bar across | ink — what they use |
| Connected standalone tool | ◈ a diamond held by two linked rings | ink — what they use |
| Dependency tool | □ a small square | ink — what they use |
| Volume | ⛁ a cylinder | the quietest words — where it runs |
| Provider | ▤ a server rack of three units | the quietest words — where it runs |
| daemon | ⊕ a globe: its ring, meridian, equator and two parallels | the quietest words — where it runs |

- **Families, not a colour each.** Three colours, all from the theme;
  the shape tells kinds apart inside a family. Gold is never a kind's:
  it marks only where you are.
- **Flat.** Every shape is a flat 2px line; nothing glows at rest.

- **Who is shown.** Every account, agent, standalone tool, connected
  standalone tool (another daemon's standalone tool, held here, listed
  by `connected_standalone_tools list`), dependency tool, volume,
  provider (the record the daemon keeps of a provider it works with,
  listed by `providers list`) and daemon (the record the daemon
  keeps of one account of ours on another daemon, listed by `daemons
  list`) the signed-in account may list. A party that appears
  only in an event, with no list holding it, is drawn too, but cannot
  be opened.
- **Live.** The board follows the daemon (`world/live.ts`): every list
  stays open.
  - **Appears:** something made — an agent, a tool, an account, a
    volume, a provider, a daemon — appears as a node without a reload, and so do its lines.
  - **Changes:** something renamed is relabelled; something attached,
    detached, mounted or messaged for the first time gains or loses its
    line.
  - **Goes:** something deleted disappears with its lines; if its sheet
    was open, the sheet closes.
- **Where new nodes land** (`grid/place.ts`, `placeOne`): below the
  lowest node of its kind, in that kind's column; the first of its kind
  starts a column of its own to the right. Nodes already on the board
  stay where they are.
- **Names.** Each node has its name beneath it, in the theme's words
  font, 12px on screen at every zoom. A tool is named by its image's
  `DIVERGE.md` `name` when the image gives one.
- **Starting places.** Nodes first stand in one column per kind
  (`grid/place.ts`).

### Live state (`grid/live.ts`)

- **Bright or faint.** Every node stands bright or faint, and the dot
  in its sheet is filled exactly when it is bright. Faint is a still
  shape, not a fade: the shape in the theme's strong edge, the name in
  its quietest words:

  | Kind | Bright when |
  |---|---|
  | Agent, standalone tool, dependency tool | its container is deployed |
  | Connected standalone tool | the daemon holds the other daemon's connect on it |
  | Account | it is connected |
  | Volume | an agent or tool it is mounted into is deployed |
  | Provider | the daemon holds a connection to it that it uses (`connected`; not one at a different version) |
  | daemon | the daemon holds a connection to it now (`connected`) |

- **Working:** an agent or tool — connected tools included — that is
  active (its loop running, or a call or read in flight) is traced: a
  line in ink draws itself round the node's own outline, then lets go
  from where it began, every 2.4 s. When the system asks for less
  motion it stands still, half drawn.
- These update live, as the daemon's lists change.

### Lines (`grid/edges.ts`, `grid/styles.ts`, `grid/links.ts`)

| Type | Looks like | Means |
|---|---|---|
| Call | solid line in the sender's family colour, chevron at the receiver | something sent to something else (an event ran that way) |
| Attachment | dashed line in the theme's strong edge, no mark | a tool or a connected tool attached to an agent (or to its template), a dependency tool to the agents that depend on it, a volume to what mounts it, a provider and what it runs (every agent and standalone tool deployed on it, every dependency tool running on it), a provider and every volume it holds, and a daemon and the connected tools on it (every connected tool whose `daemon`, its first hop, is that record; the further daemons of a tool's `route` are records on other daemons and draw nothing) |
| Creator | dotted line in the maker's family colour, open dot at the maker | who made it: the account, agent or tool that created an agent, tool, connected tool, account, provider or daemon |

- **Lanes.** Several lines between the same two nodes run side by side
  in lanes, in a fixed order, and keep that order however the nodes are
  dragged.
- **Hidden types.** Hiding a type of line closes its lanes up.

### Pulses (`grid/pulses.ts`)

- Every new event sends a short stroke in ink along its call line, from
  sender to receiver, flat — no glow — gone as it arrives.
- A burst of events sends at most a few strokes at once, spaced out.
- Pulses follow a node while it is being dragged.

### Moving around (`scenes/camera.ts`)

- **Pan:** drag the empty board with any mouse button.
- **Zoom:** the mouse wheel, from a quarter size to three times, around
  the pointer.
- **Not over the outliner:** a drag that starts on the outliner, or the
  wheel over it, does not move the view.
- **Bounds:** the view's middle can roam twice the width and height of
  the area the nodes cover, around its middle, and no further — so you
  can always reach every node however far you zoom.
- **Sharp on every screen** (`main.ts`, `lib/density.ts`): the board
  draws in the screen's own pixels, so a Retina screen gets twice the
  detail, and moving the window to another screen keeps the view. Node
  shapes are drawn four times finer than shown, so they stay sharp
  zoomed in and steady zoomed out.
- **The grid** (`scenes/backdrop.ts`) is redrawn sharp at every zoom.
  Every line is the theme's rule: whole every fifth square, at half
  strength in between, those fading out as you zoom out rather than
  turning into a haze.

### Dragging nodes (`grid/nodes.ts`)

- Press a node and move it to drag it. Its name, lines, trace, gold
  corners and pulses all go with it.
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

## Daemon tabs

`tabs/tabs.ts`, `scenes/board.ts`

- **The strip.** Along the top of the window: **Home** — our own
  daemon — first, then one tab per daemon opened. Each is named by the
  daemons along the way to it (`A › B`); hovering it shows their IDs.
  The tab shown has a gold bar along its foot.
- **Opening one.** Click a daemon's node; its sheet's **daemon** tab has
  one button, **Open <name>**. Pressing it opens that daemon in a new
  tab and shows it. A daemon opened already is switched to instead.
- **Further on.** A daemon's tab has daemon nodes of its own, the
  daemons it has records of; opening one of those goes one hop
  further (`A › B`).
- **The same app.** Inside a tab everything works as at Home — the
  board, the outliner, every sheet tab, downloads and uploads, consoles
  and frontends — on that daemon. Our daemon passes each request on
  (its `relay`, the route of daemon record IDs), connecting as our
  account there.
- **Switching.** Click a tab. The one left closes its sheet but keeps
  everything else — its lists stay open, its nodes where they were
  dragged, its view where it was — so switching back loses nothing.
  The outliner shows the tab shown.
- **Shared settings.** Which kinds and which types of line are shown is
  the same in every tab.
- **Closing.** **✕** on a tab closes it, its lists and its sheet; if it
  was shown, the tab to its left is shown. Home has no ✕.
- **Refused.** A daemon our daemon cannot reach, or the account may not
  relay to, stays "waiting for the daemon" in the outliner, under its
  header.
- **Through the strip:** a click on it never presses a node or the
  outliner beneath.

---

## The outliner

`outliner/outliner.ts`, `outliner/outliner.css`, `scenes/ui.ts`

The one menu, in HTML over the board: a card on the theme's ground with
the strong edge, at the top right at first, under the tab strip. It
takes focus and the keys like the rest of the page. A drag that starts
on it, or the wheel over it, does not move the view.

### Header

- **Outliner**, how many nodes the board holds, then three buttons:
  **Arrange** (see Arrange below), **fold**, and **dock** (or **float**,
  when docked). Folded to its header, its buttons are **unfold**, **fold
  to shapes**, **fold to a button** and **dock**.
- Under it, quietly, only when there is something to say: "waiting for
  the daemon" while the world cannot reach it, or which lists this
  account may not read.

### What to show

- **The types of line** as chips, each its sample and how many the
  board holds — calls, attachments, creators. A chip pressed (tinted)
  is shown; press it to hide or show that type, which takes its lines
  and lanes off the board.
- **The kinds** as chips, each its shape in its family's colour. Press
  one to hide or show that kind: its nodes go, and their lines and
  pulses with them; an open sheet for one of them closes, and its rows
  leave the list.

### The list

- **By family:** who acts (accounts, agents), what they use (the three
  kinds of tool), where it runs (volumes, providers, daemons). Each
  family's head shows how many it lists and folds its rows away.
- **Rows:** every node shown, by name, with its shape in its family's
  colour. A node that is not bright has its shape in the strong edge
  and its name in the quietest words.
- **The open sheet's row** is bold, on a raised band, with the gold bar
  down its left.
- **Press a row** (or Enter on it): the view flies to the node, puts the
  gold corners round it, and opens its sheet — for a provider, the
  sheet's header alone.
- **Hover or focus a row:** its band lightens and its words brighten —
  no box — and the node's lines are focused, as hovering the node itself
  does. While the keys are in use the mouse marks nothing, so only one
  row is ever marked. The arrow keys, Home and End move along the list.
- **Scrolling:** the list scrolls inside the card when it outgrows it.

### Moving, folding, docking

- **Drag the header** to move the card anywhere under the strip. Let go
  near an edge and it snaps 12px in from it. While held, its edge lights
  up.
- **Fold** shrinks it to its header; there, **unfold** opens it again,
  **fold to shapes** shrinks it to the kinds' shapes — each still a
  show/hide chip, an arrow first to open it — and **fold to a button**
  shrinks it to one button, **Outliner** and how many nodes, which opens
  it where it stands. A drag moves the button; a press opens it.
- **The shapes stand or lie:** upright down a side edge, and lying along
  the top or bottom edge when let go nearest it.
- **Resizing:** the grip in an open card's lower corner resizes it, 244
  to 420 pixels wide and at least 160 tall. On a card against the
  window's right edge the grip is in the left corner and the card grows
  leftward.
- **Opening and folding keep the edges:** a card against the right edge
  or the foot keeps that edge as it opens or folds, growing left or up.
- **Docking:** let go near the window's left edge — a dashed outline
  shows where while you hold it there — or press **dock**, and it
  becomes a rail down the left, the window's height, and the sheet opens
  beside it. Folded, the rail is the column of shapes. Drag its header
  off the rail, or press **float**, and it floats again.
- **The sheet:** a floating card stops 12px above an open sheet, or the
  window's foot, and its list scrolls in what is left.
- Where it stands and how it is folded last only for the page.

### Keys

The keys are not printed.

| Key | Does |
|---|---|
| `1` | accounts |
| `2` | agents |
| `3` | standalone tools |
| `4` | connected tools |
| `5` | daemons |
| `6` | dependency tools |
| `7` | volumes |
| `8` | providers |
| `9` | calls |
| `0` | attachments |
| `-` | creators |
| `A` | arrange |

### Arrange

- **Arrange**, or the `A` key, lays out every shown node by the shown
  lines (`grid/arrange.ts`).
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

- **Open:** click any node. Gold corners mark it, and its
  sheet opens along the bottom of the window — for a provider, which
  has no tabs, its header alone.
- **Swap:** click another node to switch the sheet to it.
- **Through the sheet:** a click on the sheet never selects a node
  beneath it.
- **Close:** click the same node again, or press **✕**.
- **Resize:** drag the handle at the top, from 20% to 90% of the window
  height.
- **Fill the window:** press **⤢** beside ✕ and the sheet, whatever tab
  is shown, covers the whole window; press **⤡** to put it back at the
  height it was. Closing the sheet puts it back too.
- **The header:** the node's name, a second line of detail (its
  template and index, the daemon a connected tool is from, who owns a
  dependency tool, an account's issuer, a provider's direction —
  outgoing to its address, or incoming — and where the daemon stands
  with it, a daemon's account of ours there and whether the
  daemon is connected to it: `as <subject> · connected`),
  and a dot, filled exactly while the node is bright on the board and a
  ring while it is not
  (see [Live state](#live-state-gridlivets)).

### Tabs by kind

| Kind | Logs | Filesystem | Console | Frontend | Daemon |
|---|---|---|---|---|---|
| Agent | yes | yes | yes | when its image declares one | — |
| Standalone tool | yes | yes | yes | when its image declares one | — |
| Connected standalone tool | yes | — | — | — | — |
| Dependency tool | yes | yes | yes | when its image declares one | — |
| Account (and admin) | yes | — | — | — | — |
| Volume | — | yes | — | — | — |
| Provider | — | — | — | — | — |
| daemon | — | — | — | — | yes |

- The tab bar always shows, even for a node with one tab (an account's
  or a connected tool's logs, a volume's filesystem).
- A connected tool's files, container and image are the other
  daemon's, so its sheet holds its log alone: every call and read this
  daemon routed into it.
- A provider has no tab at all: it has no log, files or container of
  its own here, so its sheet is its header alone.
- **A daemon's tab** holds one button, **Open <name>**, which opens
  that daemon in a tab of the window (see
  [Daemon tabs](#daemon-tabs)).
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

### Messaging an agent

`sheet/composer.ts`, `sheet/attach.ts`, `sheet/outbox.ts`

- **Writing:** an agent's log has a box under it. Enter sends;
  Shift+Enter starts a new line.
- **Files:** **Attach** adds files, any number; pasting a file, such as a
  screenshot, attaches it too. Each shows as a chip — an image as a
  thumbnail, a sound as "sound ·" and its name, anything else as
  "file ·" and its name — with ✕ to take it off.
- **What is sent:** the text first, then the files in the order they
  were attached. A file is known by its bytes, not its name, and goes
  as an image, a sound, or a file.
- **Waiting:** a message sent waits greyed at the bottom of the log
  until the log shows it — a part an account sent whose text matches
  the message's first text (or, with no text, its first block) — and
  is then gone from there, shown in the log itself. Delivery is not
  shown. Once sent, it cannot be taken back.
- **Failing:** a message the daemon refuses stays in red with why,
  until it is clicked.
- **Kept:** closing or swapping the sheet neither withdraws nor loses a
  message; it shows again when the agent's sheet next opens.
- **Permission:** the account needs the grant to message the agent.

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
- A log shorter than the sheet starts at the top, under the tabs.
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
- **Live changes:** a file or directory that changes glows briefly. A closed directory glows for changes inside it.
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

## The frontend tab

`sheet/frontend.ts`, `src-tauri/src/frontend.rs`

- **What it is.** The web page an agent's or a tool's own program
  serves, beneath
  `/frontend`, shown in a frame filling the sheet: every page, form,
  stream and live connection (WebSocket) it uses works as on any
  website.
- **Which ones.** An agent, a standalone tool or a dependency tool
  whose image's `DIVERGE.md` says `"frontend": true`. Others have no
  frontend tab. The example agent `diverge-examples/agents/human` is one: its
  frontend is where you play the agent yourself.
- **How it reaches the program.** The app's Rust serves the frame from
  an address of its own on this machine (`127.0.0.1`, a new port for
  every tab), and sends every request the page makes to the program
  through the daemon. Nothing passes through the world's page.
- **Who you are to it.** Every request is stamped by the daemon with
  the account the world is signed in as; the program reads it, and
  nothing the page sends can pass for it.
- **Frontends are kept apart.** Each frontend is a web origin of its
  own, so one page cannot read another's cookies or storage, nor reach
  into the world.
- **Only the frame gets in.** The address answers only requests from
  the frame itself (or programs on this machine, which could reach the
  daemon as you anyway); another website in a browser is refused.
- **↻** loads the frontend again from its start.
- **Starting it.** Opening an agent's or a standalone tool's frontend
  starts its container if it is stopped. A dependency tool is never
  started for it; one that is not running says so in the frame.
- **Keeping it up.** While a request or a live connection of the
  frontend is open, it keeps the container running.
- **Closing it.** Closing the sheet, opening another node, the node
  being deleted, or reloading the page ends the frontend: every
  connection it had open is closed, so it holds the container up no
  more. Switching to another tab of the same sheet keeps it.

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

- **The daemon is followed.** The world keeps the daemon's eleven lists
  open for as long as it runs (`world/live.ts`), and redraws from them
  whenever one changes, at most once a frame. It waits until every list
  has been read whole once. A list that drops is opened again 5 seconds
  later, and anything deleted while it was closed is taken off the
  board when it comes back. Every daemon tab keeps its own eleven, and
  a tab not shown keeps them open too.
- **Home's requests are unchanged.** A request at Home carries no
  `relay`; one in a daemon's tab carries the route to it.
- **The outliner's counts are live**, as the board is.
- **Reconnecting.** The connection is held by the app's Rust and redials
  by itself as `viewer.yaml` says.
- **Nothing is saved.** The app keeps no files of its own.

## Limits

These are the gaps as they stand:

- **Node positions** are lost on reload.
- **Opening a file** in place is not possible: files can be browsed,
  downloaded and uploaded, but not viewed in the app.
- **Attaching by dragging** is not possible in the log: use **Attach**
  or paste. A file is sent whole, inside the one message, however large.
- **One console per sheet.** A shell lives only while its node's sheet
  is open; there is no reattaching to an earlier one.
- **Desktop only.** Downloading and uploading need a desktop window.
