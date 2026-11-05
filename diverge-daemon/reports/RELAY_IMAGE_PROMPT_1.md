# The relay network, as one picture

An image prompt. The subject is how a request travels across Diverge
daemons: a client connected to one daemon as an account sends a request
that is served two daemons away, carried hop by hop through providers.
Accounts, daemons, providers, the records that name remote daemons,
discovery, and the grant that allows a relay all appear. The facts are
the daemon's (2026-11-05); the style is TRON.

## Style

A wide cinematic frame in the world of TRON: an endless black grid
floor receding to a dark horizon, every line glowing. A strict neon
palette on black:
- **cyan** for this daemon and the client;
- **orange** for remote daemons;
- **white** for providers;
- **magenta** for the ID tokens, the identity discs;
- thin **amber** for grants and permission checks.

Everything is built of light: glowing edges, translucent panels and
light trails, with no textures. Hard reflections on the floor. Small
monospaced holographic labels float beside each element in thin
uppercase. A title is etched in light across the top: "THE RELAY", with
a subtitle beneath: "ONE REQUEST, MANY DAEMONS". No real product names,
logos or brands.

## The cast

- **The client** is a lone program in a cyan light suit at the left edge
  of the grid, standing at a terminal pillar. On its back it wears an
  **identity disc**, a glowing magenta ring etched with `(issuer,
  subject)`. Label: "CLIENT — AN ACCOUNT".
- **Daemon A** is a towering cyan citadel at the left-center: a
  geometric fortress of stacked glowing slabs. Label: "DAEMON A — HOME".
  At its gate stands a gatekeeper program that reads every identity disc
  held up to it. Label: "HANDSHAKE — ID TOKEN VERIFIED".
- **Providers** are two white relay towers standing in the middle
  distance, tall thin pylons with rings of light around their tops.
  Labels: "PROVIDER 1", "PROVIDER 2".
- **Daemon B** is an orange citadel to the right of the providers,
  shaped like A but in orange. Label: "DAEMON B — REMOTE".
- **Daemon C** is a second, smaller orange citadel far off near the
  horizon. Label: "DAEMON C".

## The connections

1. **Daemons to providers.** Every daemon stands tethered to the
   providers by steady white beams running along the grid floor:
   daemons and providers are always connected.
2. **Discovery.** Over Daemon A floats a translucent cyan holographic map
   of the provider towers. Little orange markers show which remote daemons
   are present at which tower. Label: "DISCOVERY — WHO IS REACHABLE,
   WHERE". A faint countdown ring around the map stands for its idle
   clock: the map stays lit while it is in use and fades when idle. Daemon
   B's marker glows on Provider 2's tower, because B set its discover
   flag there.
3. **The records.** Inside A's citadel, a wall of glowing cards hangs like
   a hall of name plates. Each card reads, in monospace:
   - `bob → DAEMON B, AS ACCOUNT "ops"`
   - `bob-builder → DAEMON B, AS ACCOUNT "builder"`

   Two cards point at the **same** remote daemon, each with a different
   magenta identity disc, to show that the account matters: each card is
   a different account of ours on B. Label: "RECORDS — ONE PER ACCOUNT OF
   OURS".

## The relay, left to right

The heart of the image is one request crossing the grid as a
**lightcycle**: a sleek cyan bike leaving a solid wall of light behind it.

1. **Launch.** The client launches the lightcycle from its terminal. On
   the bike's tail a glowing tag reads `relay: ["bob", "carol"]`.
2. **A pops "bob".** The cycle passes through A's gate. The first name,
   "bob", peels off the tag as a shard of light and is caught by the
   records wall, where the `bob` card lights up. The tag now reads only
   `relay: ["carol"]`.
3. **The grant.** An amber scanning arch over A's gate checks the client's
   grant, a floating amber rune that reads `daemons: relay within
   {names: [bob]}`, and turns green. Label: "ONLY THE FIRST NAME IS
   JUDGED HERE". Beside it, a smaller arch stays dark around a second rune
   that reads `carol`, labelled "THE NEXT DAEMON JUDGES THE NEXT NAME".
4. **Picking a provider.** From the discovery map a beam picks Provider 2,
   the tower where B is present; a small die of light tumbles beside the
   pick. Label: "CHOSEN AT RANDOM AMONG THOSE THAT HOLD IT". A ghostly
   dotted path toward Provider 1 ends at nothing, because B is not
   discoverable there.
5. **Through the provider.** The lightcycle's trail bends up and over
   Provider 2's ring of light and comes down toward Daemon B. At B's gate
   it shows a **different** magenta identity disc, the one the `bob` card
   names, so A arrives at B as its account "ops". B's gatekeeper reads it
   and nods.
6. **One connection, reused.** Where A meets B, the trail becomes a
   permanent glowing conduit: a thick cyan-to-orange tube between the two
   citadels, through Provider 2. Labels: "ONE CONNECTION PER RECORD" and
   "RELAYS AND CONNECTED TOOLS SHARE IT". Inside the tube, small packets
   of light stream both ways, labelled "FRAMES, BYTE FOR BYTE".
7. **B pops "carol".** At B the last name peels off the tag. B's own amber
   arch judges it against B's grant for that account, and the cycle
   launches again, now orange, through another provider to Daemon C on
   the horizon.
8. **The answer.** The answer flows back the way it came through the
   tubes, as a gentle return pulse, to the client's terminal.

## Details to scatter in

- **A connected tool.** Inside Daemon B a small orange machine, a tool,
  is wired through the same `bob` conduit into an agent program standing
  in Daemon A's courtyard. Label: "A CONNECTED TOOL, REACHED THROUGH
  'bob'".
- **The kill switch.** In A's control room a big red-edged switch is
  labelled "ACCOUNTS DISCONNECT". One of the client figures in a far
  corner is derezzing into falling cubes of light, its line to A cut.
  Label: "CLOSED — ON DISCONNECT, EDIT OR DELETE".
- **Administrators.** A cyan figure with a crown-like crest on its
  helmet stands on A's highest tier. Label: "ADMINISTRATOR — EVERY
  GRANT".

## Composition

- **Left to right:** client, then A, then the provider towers, then B,
  then C at the horizon, so the eye follows the lightcycle's path.
- **Low camera:** the camera is low on the grid floor, looking slightly
  up at the citadels, with the long light walls running toward the
  horizon.
- **Restraint:** the labels are small and few enough not to clutter. The
  lightcycle and its tag are the brightest objects in the frame.
- **Aspect ratio:** 21:9.
