---
name: spec-prose
description: The register for writing the Diverge Provider Protocol specification (diverge-provider-web) and any normative text about the protocol — scientific, legal, academic prose in which every sentence is a complete, present-tense statement of a requirement. Load before writing or revising any specification page, protocol doc comment, or report section that states what a party must do.
---

# Specification prose

The specification is read by an implementer who will build a party
from the text alone, and by a reviewer who will hold an implementation
to it. Every sentence is therefore a finding of fact about the
protocol, written as a statute or a standard is written: complete,
literal, in the present indicative, with nothing to interpret and
nothing to forgive.

## The sentence

- **Every sentence is complete.** A subject, a finite verb, and its
  object. Never a fragment appended with a comma or a dash: not "one
  response, then the finish" but "The server sends exactly one
  response frame. The response finish follows it, and no frame
  follows the finish." Not "tag `0` and the answer, or tag `1` and an
  error" but "The payload begins with the byte `0`, followed by the
  answer, or with the byte `1`, followed by the error."
- **The verb states the act.** *Sends*, *answers*, *opens*, *closes*,
  *carries*, *precedes*, *follows*, *ignores*, *refuses*, *ends*.
  Never *then*, *next*, or *after that* standing in for a clause;
  never "X, then Y" as a sentence. Sequence is stated by verbs of
  order: "The finish follows the last response." "The request
  precedes every response."
- **Present indicative, active voice, third person.** "The server
  sends." "The proxy answers." The passive is used only when the
  actor is genuinely any party or is irrelevant: "The channel number
  is quoted, not chosen." No "we", no "you", no "let's", no "note
  that", no "of course", no rhetorical question.
- **No contractions, no colloquial elision.** "does not", "is not",
  "cannot". Not "isn't". Not "buddy-level" shorthand of any kind: not
  "one message, raw" but "one message, whose payload is the response
  frame and nothing else."
- **Exactly the quantifier.** *Exactly one*, *at most one*, *zero or
  more*, *every*, *no*, *only*. Never "a" where "exactly one" is
  meant, never "some" where "at least one" is meant, never "usually".
- **One idea per sentence; one term per idea.** A term is defined
  once and used as defined. "Scope", "channel", "request",
  "response", "finish", "ask", "answer", "path", "message", "frame",
  "payload" carry their defined meanings and no others. A synonym is
  an ambiguity.

## The requirement

- **A requirement is a statement of fact.** "A frame never spans
  messages." "Only the responder finishes a channel." RFC 2119
  keywords are not used; every sentence is already binding. A
  permission is stated with *may*; there is no *should*.
- **Say what happens, not what to do.** Not "the server should close
  the connection" but "The server closes the connection." Not "the
  client is expected to" but "The client sends."
- **State the negative where the reader could suppose otherwise.**
  "No frame follows the finish." "The request names no registry."
  "Nothing is retried." A silence the reader could fill is a
  requirement left unstated.
- **A value is stated as itself.** The literal bytes, the literal
  JSON, the literal string, with width and byte order: "the byte
  `0`", "a `u32`, big-endian", "exactly `{"type":"available"}`",
  "the string `2.3.0`". Never "the appropriate value", never "as
  above" where the value can be repeated.
- **A condition precedes its consequence.** "A payload that is not
  valid UTF-8 is malformed." "A close before the request is answered
  with the clean close and nothing before it." The condition is
  stated in full; "otherwise" refers only to the immediately
  preceding condition.
- **No reason unless the reason is a rule.** A clause of reason
  appears only when the reading depends on it: "there is no length
  field, because WebSocket already delimits messages." Motivation,
  history, and design commentary are excluded.

## The page

- **The first paragraph states the exchange in full sentences**:
  what opens it, what answers it, what ends it. It does not announce
  the page.
- **Bullets are requirements, each with a bold lead.** The lead is a
  noun phrase naming the subject ("**The payload.**", "**One
  response.**", "**Malformed.**"); the sentences after it are
  complete and stand without the lead.
- **Sequences are stated as ordered facts.** A stream is described by
  what comes first, what may repeat, what comes last, what ends it,
  and what an ending with nothing before it means — each its own
  sentence.
- **Tables enumerate values and links.** A table cell holds a value,
  a name, or a short noun phrase; it never holds a requirement.
  "Carries" columns use noun phrases ("one MCP response frame"), not
  clauses.
- **Summaries are one sentence, complete, in the same register.** A
  summary is the exchange in miniature: "A client asks what a server
  is; the server answers with exactly one response carrying the
  string `2.3.0`, and the scope finishes."
- **Diagrams in `text` blocks are annotated with noun phrases**, one
  per line, aligned: `once`, `per event`, `last`. The prose beside
  them states the sequence in sentences; the diagram never carries a
  requirement the prose does not.

## Rewrites

| Before | After |
|---|---|
| one MCP response, then the close | the server sends exactly one response frame; the close follows it |
| The answer is one message, then the close. | The proxy sends exactly one message, the answer, and closes the connection. |
| `0` ok, `1` error | The first byte is `0`, the operation done, or `1`, followed by a message stating why it was not. |
| A finish with nothing before it is the request not served. | A response finish that no response precedes states that the request was not served. |
| bytes then the close | The container sends the file's bytes as one or more body messages and closes the connection. |
| nothing sent; the opening subscribes | The server sends no message; the act of opening the path is the subscription. |
| Nothing is repeated. | No party retries the operation. |
| It just closes. | The proxy closes the connection and sends no message. |

## Before writing

Read the page's subject in the crate. State only what the crate and
the user's rulings establish. Where the crate is silent, do not
write; report the gap. Where a sentence could be read two ways,
rewrite it until it can be read one way.
