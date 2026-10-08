# Image prompt: the accounts system

One picture of how Diverge is governed: every agent is a robot, every
tool is a wrench, and who may do what is a system of accounts and
roles, drawn in the style of a civilization strategy game.

## The prompt

A wide isometric illustration in the style of a classic civilization
strategy game: a hex-tiled landscape seen from above at a slight
angle, painted with clean saturated colors, soft directional
lighting, and the polished look of a game's empire-overview screen.
No photorealism; warm, readable, diagrammatic game art.

The land is divided into a few distinct territories, each a cluster
of hex tiles in its own tint — one amber, one teal, one violet, one
olive. Each territory is an ACCOUNT. At the center of each territory
stands a small stone fortress with a banner: the banner carries one
simple heraldic emblem and no words. Around each fortress live one or
more robots. Every robot is an AGENT: a friendly, rounded, humanoid
robot with a glowing visor, each robot visibly different in color
and build from the others.

Every robot carries wrenches on a tool belt and on its back. The
wrenches are the robot's TOOLS, and no two robots carry the same set:
one robot has three large copper wrenches, another a single long
silver wrench, another a bundle of small brass wrenches, one carries
only a tiny wrench, and one carries none. Each wrench has a small
distinct engraved icon on its head — a gear, a leaf, a flame, a
wave, a key — so that a viewer can see which capabilities each robot
holds without any label. The robots are plainly specialists: their
wrenches say what each can do.

Between the robots run narrow glowing roads laid over the hex tiles,
in the way a strategy game draws trade routes. A road between two
robots means those two may send messages to each other. Some robots
are joined by many roads, some by one, and two robots stand on an
island of tiles with no road leading to anyone. Where a road crosses
from one territory into another, it passes through an open gate in a
low wall; where there is no permission, the wall has no gate and the
road ends at it.

Above each fortress floats a translucent scroll, drawn as a game's
civic or policy card. Each scroll shows a column of small pictograms
and nothing else: a wrench, a robot, an envelope, a gate, a scroll —
each pictogram either lit in gold or dimmed in grey. These are the
ROLES: a lit pictogram is a grant held, a dimmed one is a grant not
held. The scrolls are the only place the system's rules appear, and
they are shown entirely in pictograms.

At the top of the image, in the exact frame of a strategy game's
header bar, a single line of text in a clean serif game font reads:

    Every robot is an account. Its wrenches are what it may do; its roads are whom it may reach.

The image contains no other text of any kind: no labels on the
territories, no names on the robots, no words on the banners, the
wrenches, the roads, the scrolls, the walls, or the gates. Every
other meaning is carried by shape, color, position, and light.

Composition: the whole scene fits in one frame with generous margin,
the richest territory slightly left of center, the lone island at
the lower right, the header bar across the top edge. Aspect ratio
16:9.

## What each thing stands for

| In the picture | In the daemon |
|---|---|
| A territory and its fortress | An account |
| A robot | An agent, which is an account by its `account` |
| A wrench on a robot | A tool the agent has — attached to it, or deployed for it |
| The engraved icon on a wrench | What the tool does; the capability itself |
| A road between two robots | A `message` grant reaching from one agent's account to the other |
| A gate in a wall | A grant that crosses accounts; no gate, no grant |
| A scroll of pictograms | A role: the list of grants an account holds |
| A lit pictogram | A grant held; a dimmed one, not held |
| The island with no road | An agent whose account may message nobody |
