import { SharedHeader } from "@/components/SharedHeader";
import { SharedFooter } from "@/components/SharedFooter";
import { MarkdownContent } from "@/components/Markdown";
import { Provider } from "@/provider";
import { ReactElement } from "react";
import cn from "classnames";
import { Sidebar } from "./Sidebar";

const content = `## What is ObjectiveAI?

ObjectiveAI is a REST API ([api.objective-ai.io](https://api.objective-ai.io)) and platform for **scoring, ranking, and simulating decisions** using large language models.

Instead of generating free-form text, ObjectiveAI focuses on producing **structured outputs**:
- scores
- rankings

These outputs can be trained from data, reused, and composed into larger decision systems.

---

## What does ObjectiveAI actually do?

ObjectiveAI lets you:
- score inputs (for example: "how good is this?")
- rank options ("which of these is best?")
- simulate preferences ("how would this person or system decide?")

It does this using **ensembles of LLMs** with explicit, learnable weights.

---

## What are the main features?

ObjectiveAI provides three core capabilities:

- **Chat Completions** - standard LLM chat, with reusable model configurations
- **Vector Completions** - turn LLM judgments into numeric score vectors
- **Functions** - composable, trainable scoring pipelines (what most users use)

---

## What is a Chat Completion?

A Chat Completion is a normal LLM chat request: messages in, text out.

The difference is that ObjectiveAI lets you call **configured model definitions**, which may include:
- built-in prefix or suffix prompts (for personality or instructions)
- decoding and provider settings

From the user's perspective, it behaves like a standard chat completion.

---

## What is a Vector Completion?

A Vector Completion produces **numbers**, not text.

It works by:
1. Running multiple Chat Completions (one per model in an ensemble)
2. Forcing each completion to produce a structured **vote** for a potential response. Each response is user-provided.
3. Combining those votes using explicit weights
4. Returning a **vector of scores** that sums to 1. The winning response is the one with the highest score.

Vector Completions are used for:
- ranking options
- selecting winners
- producing machine-usable scoring outputs

---

## What is a Function?

A **Function** is a reusable scoring pipeline.

Conceptually:

> **input -> score(s)**

A Function:
- accepts structured input
- runs a list of tasks
- each task is either a Vector Completion or another Function
- produces either:
  - a single score, or
  - a vector of scores

Functions can be simple or complex, public or private, and are immutable and versioned.

---

## Why would I use a Function instead of Chat or Vector Completions directly?

Because Functions are simpler and safer.

With Functions:
- you don't manage prompts per request
- you don't wire together ensembles yourself
- you don't handle voting logic
- you don't interpret raw vectors manually

You just send input and get back a score or ranking.

---

## Can Functions be trained?

Yes.

Functions can be trained by providing **example inputs and desired outputs**.

ObjectiveAI:
- keeps the underlying models fixed
- learns **weights** over ensembles
- caches AI outputs so optimization is CPU-bound
- produces a **Profile**, which captures the learned behavior

This avoids fine-tuning models while still adapting behavior to your use case.

---

## What is a Profile?

A **Profile** is a trained configuration for a Function.

It defines:
- which ensembles are used
- what weights are applied
- how the Function behaves in practice

Profiles are:
- immutable
- versioned
- reusable
- shareable

A Function always has a default Profile, and you can publish new ones as you train.

---

## Are you asking LLMs to produce scores?

No.

LLMs are not asked to produce numbers at any stage whatsoever.

Instead:
- each model produces a vote
- votes may be probabilistic by leveraging logprobs
- votes are combined externally
- behavior is learned from data, not self-reported confidence

This keeps the system simple, robust, and model-agnostic.

---

## What is an Ensemble?

An **Ensemble** is a collection of configured LLMs used together.

Important properties:
- Ensembles are immutable
- Ensembles do not contain weights
- Any change creates a new Ensemble ID
- The same Ensemble can support many different behaviors via different Profiles

---

## What are weights?

Weights control how much influence each model has during voting.

They are:
- explicit
- external to Ensembles
- static per execution
- learned during training

Weights are never hidden or implicit.

---

## What LLMs are supported?

ObjectiveAI supports any LLM available through its upstream providers.

Ensembles can mix:
- different model families
- different providers
- different configurations

The system is intentionally provider-agnostic.

---

## How much does ObjectiveAI cost?

ObjectiveAI uses a credits-based pricing model.

Costs depend on:
- the models used
- the number of tokens processed
- the size of ensembles
- whether training is involved

Pricing reflects upstream model costs plus a service fee.

For custom pricing, please contact us.

---

## Who is ObjectiveAI for?

ObjectiveAI is for:
- developers building ranking or scoring systems
- teams simulating preferences or decisions
- products that need consistent, reproducible AI judgments

You do **not** need to be an ML expert to use it.

It is **decision infrastructure**.

---

## Where can I learn more?

Documentation and examples are available on our website, and the open-source components live on GitHub.

If you're interested in **ranking, scoring, or simulating decisions at scale**, ObjectiveAI is built for that.`;

export function FAQ({
  session,
}: {
  session?: Provider.TokenSession;
}): ReactElement {
  return (
    <main className={cn("h-[100dvh]", "w-[100dvw]", "flex", "flex-col")}>
      <SharedHeader session={session} />
      <div className={cn("flex-grow", "flex", "overflow-hidden", "mt-1")}>
        <Sidebar />
        <div
          className={cn(
            "flex-grow",
            "flex",
            "flex-col",
            "overflow-auto",
            "basis-0",
            "px-1"
          )}
        >
          <div
            className={cn(
              "w-[calc(var(--spacing)*192)]",
              "max-w-full",
              "space-y-8",
              "mx-auto",
              "my-8"
            )}
          >
            <div className={cn("space-y-4")}>
              <h3
                className={cn(
                  "text-2xl",
                  "sm:text-3xl",
                  "font-medium",
                  "border-b",
                  "border-muted-secondary",
                  "pb-4"
                )}
              >
                FAQ
              </h3>
              <MarkdownContent content={content} />
            </div>
          </div>
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}
