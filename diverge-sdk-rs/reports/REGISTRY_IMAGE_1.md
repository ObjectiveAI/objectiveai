# Image prompt: the peer-to-peer image registry, as a 1950s advertisement drawn in rubber-hose

An image prompt for one picture of what the image system is on
2026-10-11: daemons OWN images; an agent makes an image on the spot,
from a recipe, and runs it the moment it exists; a daemon hands an
image from one provider to another without touching the bytes; and an
image goes out to the world's registries when somebody wants it there,
with one gesture. The picture is a full-page magazine advertisement,
the kind that sold refrigerators and station wagons, drawn by an
animation studio from the rubber-hose era. Who owns what and what
flows where is fixed in the tables below. How it is drawn is the
image's to decide, within the tone.

## Prompt

### The tone

A full-page colour advertisement from a 1950s American magazine,
filling the frame edge to edge: the page itself, slightly yellowed,
with the halftone dot of a four-colour press visible up close and a
faint show-through of the page behind. The layout is an ad layout — a
confident headline across the top in a hand-lettered script with a
drop shadow, a big central illustration, two or three small inset
panels down one side like a how-it-works strip, and a line of small
print along the bottom. A single starburst in one corner says `NEW!`
and nothing else. The palette is advertising pastel over warm paper:
cream, mint, butter yellow, coral, powder blue, one cherry red for
emphasis, black line throughout, with the slight misregistration of
the colour plates against the black.

Every character is drawn in rubber-hose animation: round heads on
noodle limbs with no elbows and no knees, three-fingered white gloves,
pie-cut pupils, big flexible shoes, and bodies that squash and stretch
with whatever they are doing. Machines have faces too — not cute
faces, working faces: a furnace that grins while it works, a shelf
with eyebrows. Everything bounces. Nothing is static, even in a still
image; every pose is the top of a bounce or the bottom of a squash.
Motion lines, sweat drops of effort, little puffs of exertion, a
tongue out in concentration. The drawing is clean ink line, no
sketchiness, no crosshatching: flat colour, bold outline, the
confidence of a studio that has drawn ten thousand of these.

No photographs, no gradients, no lens flare, no modern UI, no screens
with text on them, no logos, no brand names, no real companies. Text
on the page is only what this prompt names, and it is lettered, never
typeset in a modern face.

**The headline, hand-lettered in cherry red script across the top
with a butter-yellow drop shadow: `Your Images, Everywhere You Are!`**

### The cast

| Character | Who it is | How it is drawn |
|---|---|---|
| a DAEMON | the one who OWNS images, the client of every provider | a tall, friendly fellow in a conductor's cap and a vest with many pockets, each pocket holding a labelled tin; he never carries a crate himself, he points and things move |
| a PROVIDER | a machine that holds images and runs containers | a squat, jolly cast-iron furnace-and-cupboard with a stovepipe hat, a round dial for a face, a door in its belly that opens on shelves of tins, and a chimney that puffs a ring when it works |
| an AGENT | a program running in a container, which makes images on the spot | a small sparky character in overalls and goggles, three-fingered gloves, standing on a workbench, cheerfully dangerous |
| a TIN | one image, held by digest | a round tin can with a label that is a plain wax seal stamped with a short hexadecimal, no picture, no brand; the SAME tin looks the same wherever it sits |
| the WORLD REGISTRY | the public registries, where an image goes when published | a grand department-store shelf far at the back, columns and awnings, a doorman, thousands of tins on its shelves in neat rows, a revolving door |

Every tin on the page that is the same image bears the same seal.
There are three images in the picture and the viewer should be able
to find each one wherever it is by its seal: `9F2C`, `A41E`, `3D07`.

### The central illustration

A bright kitchen-and-workshop interior, cut away like a product
diagram, the floor a checkerboard in mint and cream. Three providers
stand along the floor like three appliances in a showroom, left to
right, each with its belly door open:

| Provider | Hat | What is on its shelves |
|---|---|---|
| `I`, left | a short top hat | tin `9F2C`, tin `A41E` |
| `II`, middle | a bowler | tin `3D07`, and one empty slot with a dotted outline |
| `III`, right | a chef's toque | nothing on its own shelf yet; a label on its door reads `COMING RIGHT UP` |

The daemon stands in the middle of the floor, feet planted, one
glove on his hip, the other glove pointing. He owns every tin on the
page. The tins on the providers' shelves each have a tiny tag on a
string tied to the daemon's vest pocket — thin black lines, like the
strings on helium balloons — so the ownership is drawn, not implied.
Three strings to Provider `I`'s and `II`'s tins; none yet to `III`.

### The flows, which are the point

Four things are happening at once, and each is one motion line.

**Making an image on the spot.** On top of Provider `III`, on its
flat iron lid as a workbench, the AGENT is cooking. A recipe card is
pinned up beside the agent, hand-lettered `RECIPE` with three
illegible lines of script under it — a Containerfile, never spelled
out. Beside the card is a small open pantry box labelled `CONTEXT`
with folders and a cog spilling out; the agent is throwing those into
a mixing bowl. Out of the bowl, hot, with motion lines and a puff,
comes a NEW tin, seal `3D07` — the same seal as the tin on Provider
`II`'s shelf, because it is the same image — and Provider `III`'s belly
door is already opening to receive it. A tiny stopwatch over the
agent's head reads a few seconds. The agent is drawn mid-stretch,
both arms long, one foot off the bench.

**Running it the instant it exists.** Beside the mixing bowl, the
same agent's other glove is already pulling a cord, and from Provider
`III`'s chimney a second, smaller rubber-hose character is popping
out in a puff of steam — a brand new running container, eyes just
opening, wearing a label `3D07` on its chest. No pause between the
tin and the running thing: one squash-and-stretch.

**Handing an image from one provider to another.** Provider `II` is
asking for tin `9F2C` — a thought-balloon over its dial-face shows the
seal `9F2C` with a question mark. The daemon does not fetch it
himself. He has unreeled a garden hose from Provider `I`'s belly to
Provider `II`'s belly, holding the middle of the hose in one glove,
and tin `9F2C` is travelling down the hose as a bulge, the way a
rubber-hose cartoon moves anything through a tube. The daemon's eyes
are closed; he is whistling. A small label on the hose, lettered on a
paper tag: `NO HANDS`. The dotted empty slot on Provider `II`'s shelf
is where it will land.

**Publishing to the world.** From Provider `I`'s stovepipe hat a
pneumatic tube runs up and across the top of the room, bending
rubber-hose-style, and out the back wall to the WORLD REGISTRY, which
is drawn through a big open window as a department store across a
street at dusk. Tin `A41E` is in the tube, halfway. The daemon's
pointing glove is pointing at it: one gesture, the tin goes. At the
store, the doorman tips his hat and a shelf-clerk on a ladder is
already lifting the previous `A41E` from the tube's mouth onto the
shelf. Lettered on the store's awning, small: `EVERYONE'S`. Nothing
else on the store.

### The inset strip, down the right side

Three small panels in a column, each with a one-line caption
hand-lettered beneath it, in the register of an ad's how-it-works:

1. The agent at the bowl, the tin popping out. Caption:
   `1. Cook it up.`
2. The daemon pointing, the tin going down the hose between two
   providers. Caption: `2. Hand it along.`
3. The tube to the department store, tin in flight. Caption:
   `3. Send it out — when YOU say.`

### The small print, along the bottom

One line, lettered small and even, the only other sentence on the
page: `Every tin is yours. Every furnace can hold it. Nobody opens it
but you.` Nothing after it.

### The tally, so the picture is right

- Three images, three seals, each the same wherever it sits: `9F2C`
  on Provider `I` and in the hose; `A41E` on Provider `I` and in the
  tube and on the store's shelf; `3D07` on Provider `II`'s shelf and
  coming out of the agent's bowl.
- One daemon, one agent, three providers, one store, one running
  container popping out of chimney `III`.
- Strings from every provider-held tin to the daemon's pocket.
  None from the store's shelf: once it is published it is the
  world's, and the string is cut — a small snipped string hangs off
  the tube's end at the store.
- The daemon's hands touch no tin. The hose and the tube and the
  pointing glove do the moving.
- The agent is the only one in a hurry.

### Everything else

Everything not stated is the image's to decide: the exact pose of
each character, how the room is cut away, the angle on the store,
how the plates misregister, how worn the page is, the lettering
style within "1950s hand-lettered script", the exact pastel of each
appliance. Keep it to one page, one illustration, three insets, one
headline, one starburst, one line of small print, five labels
(`RECIPE`, `CONTEXT`, `COMING RIGHT UP`, `NO HANDS`, `EVERYONE'S`),
and three seals. No other text anywhere.

## The cast, against the system

For whoever reads the picture back into the protocol.

| In the picture | In the system |
|---|---|
| the daemon owning every tin, strings to his pocket | a daemon is the client of every provider; an image an identity built on a provider is held for that identity and for no other (`images::build`, `images::list`, `images::stat`, `images::delete`) |
| the seal on a tin | the image's manifest digest: the same bytes are the same image wherever they are, and a name is only a locator |
| the agent cooking from the recipe card and the pantry box | `images::build`: a Containerfile the request carries, with a directory of one of the caller's volumes as the build's context and working directory |
| the container popping out of the chimney at once | a run naming the digest just built: the provider finds it in what it holds and runs it with nothing pulled |
| the hose from Provider `I` to Provider `II`, the daemon whistling | a run on `II` asks the daemon whether it holds `9F2C` (`oci-has` on the run scope); the daemon asks every other provider (`images::check`, sources `client` and `server`), finds `I`, and relays the manifest and blobs from `I`'s `images::oci` scope to `II`'s run scope, reading nothing |
| the dotted slot on `II`'s shelf | `II`'s own registry on its loopback, which podman pulls from, filled by digest as the daemon relays |
| the pneumatic tube to the department store | publishing to a registry the provider's policy lists, under a reference's registry and name — the one gesture, when wanted, not by default |
| the snipped string at the store | a published image is the registry's to serve; a reference locates it, and the digest still identifies it |
| `COMING RIGHT UP` on Provider `III` | a provider whose store is a stub today: the protocol is in place and the shelves are being built |
