# Image prompt: the daemon's ACL, and doing things through other agents

An image prompt for one picture of the access-control system in
report 2. The primary subject is delegation: an agent's own tools are
hard-coded on its create, and whether one agent may message another
is itself a permission, so an agent reaches, through the agents it
may message, whatever those agents may do. Everything else in the
system appears around that.

## Prompt

A wide technical illustration, landscape, clean vector style on a
dark slate background, thin light lines, a restrained palette: one
cool colour for agents, one warm colour for tools, a neutral grey for
templates, white for the client, and a single accent colour, amber,
reserved for permission edges. Think of a well-made systems diagram
drawn as a poster: flat, precise, legible at a glance, no 3D, no
photorealism, no people.

**The centre of the picture is three agents, drawn as rounded
hexagonal nodes in a loose triangle, not a line.** They are peers:
no agent is above another, and nothing about their shape or size
differs. Each is labelled only by what it is, its template's short
hash and its count, in monospace: `3f9a…#1`, `3f9a…#2`, `b17c…#1` —
the first two made from the same template, the third from another.
A thin grey line runs from each node up to the template card it was
made from. Each node has a
small locked panel bolted to its side, like a riveted plate, labelled
`daemon_tools`, holding a short list of tiny icons: a list glyph, an
envelope, a scroll, a plus, a trash can, a pencil, a tag. On each
agent only SOME of the icons are lit; the rest are dim. A small
padlock and the words "set at create, for life" sit under every
plate: the tools are hard-coded, nobody edits them later.

- `3f9a…#1` has only two icons lit: the envelope
  (`agents_message`) and the list glyph (`agents_list`). Its
  envelope icon is drawn larger than the others.
- `3f9a…#2`, made from the SAME template, has a different set lit:
  the envelope and the plus (`agents_create`). A small note between
  the two reads "same template, different daemon_tools: the tools
  are the create's, not the template's".
- `b17c…#1` has the trash can (`agents_delete`) and the tag
  (`agents_tag`) lit, and no envelope at all.

**Between the agents run thick amber arrows, the permission edges,
each passing through a sieve.** The sieve is a small rectangular
frame with a mesh, labelled `filter`, and the mesh carries a short
inscription in monospace: on the first arrow `all_tags: [crew]`, on
the second `any_tags: [crew] · jq: .active`. Below each sieve a
caption reads "passes → may message". The arrow from `3f9a…#1` to
`3f9a…#2` is an envelope in flight; the arrow from `3f9a…#2` to
`b17c…#1` is another. Where an arrow reaches an agent, a faint dotted
line continues from that agent's lit icons onward, so the eye reads
that `3f9a…#1`, with nothing but an envelope, ends up with a plus, a
trash can and a tag by way of the others. A single bold caption
above the three, in the accent colour, states the point: **"An agent
does through others what it may not do itself: the envelope is the
key to every other tool."** A smaller line beneath it: "who may
message whom is a filter on each agent's create; there are no roles,
only reach".

Also draw, faintly, the arrows that do NOT exist: a greyed,
crossed-out envelope from `b17c…#1` back toward `3f9a…#1`, and
another from `3f9a…#1` straight to `b17c…#1`, each with the caption
"no `agents_message` passes → cannot reach", to show the edges are
one-way, granted and not assumed, and that the only way from the
first agent to the third is through the second.

**Above the chain, at the top of the picture, the client.** A white
circle labelled `client`, with a small identity tag reading
`identity: …`. From it descend thin white lines to the roots of
everything below: it is the first link of every creator chain. Beside
each agent, a tiny breadcrumb strip reads its `creator` chain — the
plus icon on `3f9a…#2` is what made `b17c…#1`, and the breadcrumb
says so — for
instance `client › 3f9a…#1 › 3f9a…#2`, where the number after the
hash is the agent's `count` under its template, and a small
legend explains "template + count names it once and for all; the
name may be reused after deletion". One agent node is drawn with a
dashed outline and the label `deleted`, its breadcrumb still intact,
to show the chain survives.

**On the left margin, the templates.** Two grey card stacks, one
labelled `agent templates`, one `tool templates`, each card showing a
`type` ribbon (`agent` or `tool`), an image name, two small bars for
memory and disk, and a hash printed along the bottom edge in
monospace, `sha256: 3f9a…`. A note on the stack: "hashed; tags and
daemon_tools live outside the hash". Thin grey lines run from a card
to each agent made from it, and the `count` on the breadcrumb ties
back to that card. One card carries two small tag chips to show
templates are tagged too.

**On the right margin, the tool containers.** Warm-coloured square
nodes labelled `tools`, each with a small MCP plug symbol. Two are
plugged by short warm lines into `b17c…#1` and `3f9a…#2`, labelled
`attach`. One tool node is drawn with a hollow outline and a distant
dotted line leaving the picture, labelled `connected`, with the note
"somebody else's container: holds no daemon_tools, is never a
creator". Below the tools, a small shelf of document icons labelled
`resources`, each with a hash, with a thin line from one document to
an agent, labelled `fuse`.

**Along the bottom, a legend strip of the remaining vocabulary, each
item a small icon with a label**, in this order:

1. A tag chip, `tags`: "tag / untag endpoints; `all_tags` every one
   of, `any_tags` any one of".
2. A sieve, `Filter`: "the same shape narrows a list and grants a
   tool; as a grant, the jq program is a test: truthy passes".
3. A pair of name cards, `reference`: "`{name}` reaches it now;
   `{template, count}` reaches it once and for all".
4. A padlock on a plate, `daemon_tools`: "29 tools, one per endpoint
   but `connect`; filters, booleans for creates and upload, `any` or
   `only` for tags and resources".
5. A breadcrumb strip, `creator`: "client first, maker last; lists
   narrow by any link in the chain".

Typography: a clean geometric sans for labels, a monospace face for
every identifier, hash and filter inscription. Keep every label
short; the picture carries the meaning. Leave generous empty space
around the central three so they dominate. No logos, no watermark, no
text other than what is specified here.

## Negative prompt

Photographs, people, hands, robots with faces, cartoon characters,
3D rendering, glossy gradients, neon glow, clutter, more than five
colours, paragraphs of text inside the image, misspelled labels,
org charts, crowns, ranks, or any label that names a role such as
"manager", "boss", "planner" or "worker".
