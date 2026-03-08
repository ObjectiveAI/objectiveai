"use client";

import { useState, useEffect, use } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { createPublicClient } from "../../../lib/client";
import { deriveDisplayName, DEV_EXECUTION_OPTIONS } from "../../../lib/objectiveai";
import { PINNED_COLOR_ANIMATION_MS } from "../../../lib/constants";
import { DEFAULT_PROFILES } from "../../../lib/profiles";
import { loadReasoningModels } from "../../../lib/reasoning-models";
import { useIsMobile } from "../../../hooks/useIsMobile";
import { useObjectiveAI } from "../../../hooks/useObjectiveAI";
import { InputBuilder } from "../../../components/InputBuilder";
import SchemaFormBuilder from "../../../components/SchemaForm/SchemaFormBuilder";
import type { InputSchema, InputValue } from "../../../components/SchemaForm/types";
import SplitItemDisplay from "../../../components/SplitItemDisplay";
import { simplifySplitItems, toDisplayItem, getDisplayMode } from "../../../lib/split-item-utils";
import { compileFunctionInputSplit, type FunctionConfig } from "../../../lib/wasm-validation";
import { Functions, EnsembleLlm } from "objectiveai";
import { ObjectiveAIFetchError } from "objectiveai";
import { SkeletonFunctionDetails } from "../../../components/ui";

interface FunctionDetails {
  owner: string;
  repository: string;
  commit: string;
  name: string;
  description: string;
  category: string;
  type: "scalar.function" | "vector.function";
  inputSchema: Record<string, unknown> | null;
}

export default function FunctionDetailPage({ params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = use(params);

  const router = useRouter();

  // Parse slug: catch-all route gives us string[] e.g. ["owner", "repo"]
  const owner = slug[0] || "unknown";
  const repository = slug.length >= 2 ? slug[1] : slug[0] || "unknown";

  // Backward compat: old "--" URLs redirect to new "/" format
  useEffect(() => {
    if (slug.length === 1 && slug[0].includes("--")) {
      router.replace(`/functions/${slug[0].replace("--", "/")}`);
    }
  }, [slug, router]);

  // Canonical key for localStorage pinning (owner/repo format)
  const slugKey = `${owner}/${repository}`;

  const [functionDetails, setFunctionDetails] = useState<FunctionDetails | null>(null);
  const [selectedProfileIndex, setSelectedProfileIndex] = useState(0);
  const [availableProfiles, setAvailableProfiles] = useState<Array<{
    owner: string;
    repository: string;
    commit: string | null;
    label: string;
    description: string;
  }>>(DEFAULT_PROFILES);
  const [isLoadingDetails, setIsLoadingDetails] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [formData, setFormData] = useState<InputValue>({});
  const [isRunning, setIsRunning] = useState(false);
  const isMobile = useIsMobile();
  const { getClient } = useObjectiveAI();
  const [isSaved, setIsSaved] = useState(false);
  const [showPinnedColor, setShowPinnedColor] = useState(false);
  const [splitItems, setSplitItems] = useState<InputValue[] | null>(null);
  const [results, setResults] = useState<{
    output?: number | number[];
    inputSnapshot?: Record<string, unknown>; // Store input for display
    usage?: {
      prompt_tokens: number;
      completion_tokens: number;
      total_tokens: number;
      cost?: number;
      total_cost?: number;
    };
    tasks?: Array<{
      votes?: Array<{
        model: string;
        vote: number[];
        weight: number;
        from_cache?: boolean;
        from_rng?: boolean;
      }>;
      completions?: Array<{
        model: string;
        choices?: Array<{
          message?: {
            content?: string;
          };
          delta?: {
            content?: string;
          };
        }>;
      }>;
      scores?: number[];
    }>;
    reasoning?: {
      choices?: Array<{
        message?: {
          content?: string;
        };
      }>;
    } | null;
    error?: string;
  } | null>(null);
  const [runError, setRunError] = useState<string | null>(null);
  const [modelNames, setModelNames] = useState<Record<string, string>>({});
  const [showAllModels, setShowAllModels] = useState(false);
  const [expandedVotes, setExpandedVotes] = useState<Set<number>>(new Set());

  // Demo mode: when enabled, uses RNG votes (free, simulated). When disabled, uses real LLM inference.
  const [demoMode, setDemoMode] = useState(true);

  // Reasoning options
  const [reasoningEnabled, setReasoningEnabled] = useState(false);
  const [reasoningModel, setReasoningModel] = useState(""); // Set after loading from JSON
  const [reasoningModels, setReasoningModels] = useState<Array<{ value: string; label: string }>>([]);

  // Fetch function details
  useEffect(() => {
    // Skip fetching if this is a legacy "--" URL that will redirect
    if (slug.length === 1 && slug[0].includes("--")) return;

    async function fetchDetails() {
      try {
        setIsLoadingDetails(true);
        setLoadError(null);

        const publicClient = createPublicClient();

        // Fetch function details directly (works for all functions, regardless of profiles)
        const details = await Functions.retrieve(publicClient, "github", owner, repository, null);

        const category = details.type === "vector.function" ? "Ranking" : "Scoring";

        setFunctionDetails({
          owner,
          repository,
          commit: details.commit || "",
          name: deriveDisplayName(repository),
          description: details.description || `${deriveDisplayName(repository)} function`,
          category,
          type: details.type as "scalar.function" | "vector.function",
          inputSchema: (details as { input_schema?: Record<string, unknown> }).input_schema || null,
        });

        // Try to get available profiles (separately, so function loads even if no profiles exist)
        let functionProfiles: Array<{ owner: string; repository: string; commit: string; label: string; description: string }> = [];
        try {
          const pairs = await Functions.listPairs(publicClient);
          const matchingPairs = pairs.data.filter(
            (p: { function: { owner: string; repository: string } }) =>
              p.function.owner === owner && p.function.repository === repository
          );
          functionProfiles = matchingPairs.map(
            (p: { profile: { owner: string; repository: string; commit: string } }) => ({
              owner: p.profile.owner,
              repository: p.profile.repository,
              commit: p.profile.commit,
              label: deriveDisplayName(p.profile.repository),
              description: `${p.profile.owner}/${p.profile.repository}`,
            })
          );
        } catch {
          // If pairs fetch fails, continue to fallback
          functionProfiles = [];
        }

        // Fallback: try fetching profile from same repo (CLI puts profile.json in the function repo)
        if (functionProfiles.length === 0) {
          try {
            const profile = await Functions.Profiles.retrieve(publicClient, "github", owner, repository, null);
            functionProfiles = [{
              owner,
              repository,
              commit: profile.commit,
              label: deriveDisplayName(repository),
              description: `${owner}/${repository}`,
            }];
          } catch {
            // Genuinely no profile exists for this function
          }
        }

        // Function-specific profiles first, then defaults
        setAvailableProfiles([...functionProfiles, ...DEFAULT_PROFILES]);
        setSelectedProfileIndex(0);
      } catch (err) {
        setLoadError(err instanceof Error ? err.message : "Failed to load function");
      } finally {
        setIsLoadingDetails(false);
      }
    }

    fetchDetails();
  }, [owner, repository]);

  // Load saved state from localStorage + migrate old "--" keys
  useEffect(() => {
    const savedLibrary = localStorage.getItem("pinned-functions");
    if (savedLibrary) {
      const library: string[] = JSON.parse(savedLibrary);
      // Migrate old "--" format keys to "/" format
      const migrated = library.map((s: string) => s.includes("--") ? s.replace("--", "/") : s);
      if (JSON.stringify(migrated) !== JSON.stringify(library)) {
        localStorage.setItem("pinned-functions", JSON.stringify(migrated));
      }
      setIsSaved(migrated.includes(slugKey));
    }
  }, [slugKey]);

  // Load reasoning models from build-time generated JSON
  useEffect(() => {
    loadReasoningModels().then(config => {
      setReasoningModels(config.models.map(m => ({ value: m.value, label: m.label })));
      setReasoningModel(config.default_model);
    });
  }, []);

  // Toggle save state
  const toggleSave = () => {
    const savedLibrary = localStorage.getItem("pinned-functions");
    const library = savedLibrary ? JSON.parse(savedLibrary) : [];

    if (isSaved) {
      const updated = library.filter((s: string) => s !== slugKey);
      localStorage.setItem("pinned-functions", JSON.stringify(updated));
      setIsSaved(false);
    } else {
      library.push(slugKey);
      localStorage.setItem("pinned-functions", JSON.stringify(library));
      setIsSaved(true);
      setShowPinnedColor(true);
      setTimeout(() => setShowPinnedColor(false), PINNED_COLOR_ANIMATION_MS);
    }
  };

  // Fetch model names when results contain votes using SDK
  useEffect(() => {
    if (!results?.tasks || !Array.isArray(results.tasks) || results.tasks.length === 0) return;

    const allVotes = results.tasks.flatMap(t => (t && t.votes) ? t.votes : []);
    if (allVotes.length === 0) return;

    const uniqueIds = [...new Set(allVotes.filter(v => v?.model).map(v => v.model))];
    const idsToFetch = uniqueIds.filter(id => id && !modelNames[id]);

    if (idsToFetch.length === 0) return;

    // Fetch in parallel using SDK
    (async () => {
      const client = await getClient();
      const fetchResults = await Promise.all(
        idsToFetch.map(async (id) => {
          try {
            const llm = await EnsembleLlm.retrieve(client, id);
            return { id, model: llm.model as string };
          } catch {
            // Ignore errors, fall back to cryptic ID
            return null;
          }
        })
      );
      const newNames: Record<string, string> = {};
      for (const r of fetchResults) {
        if (r) newNames[r.id] = r.model;
      }
      if (Object.keys(newNames).length > 0) {
        setModelNames(prev => ({ ...prev, ...newNames }));
      }
    })();
  }, [results?.tasks, modelNames, getClient]);

  // Compute split items for vector results visualization
  useEffect(() => {
    if (!results?.output || !Array.isArray(results.output) || !functionDetails) return;

    // Capture values for the async function
    const inputSnapshot = results.inputSnapshot;
    const { owner, repository, commit } = functionDetails;

    async function computeSplitItems() {
      try {
        // Fetch the full function definition for WASM compilation
        const publicClient = createPublicClient();
        const funcDef = await Functions.retrieve(publicClient, "github", owner, repository, commit);

        // Use WASM to compile the input split
        const splitResult = await compileFunctionInputSplit(funcDef as unknown as FunctionConfig, inputSnapshot);
        if (splitResult.success && splitResult.data) {
          // Simplify the items for display (cast to InputValue[])
          const simplified = simplifySplitItems(splitResult.data as InputValue[]);
          setSplitItems(simplified);
        }
      } catch {
        // Keep splitItems as null, will fall back to basic labels
      }
    }

    computeSplitItems();
  }, [results?.output, results?.inputSnapshot, functionDetails]);

  /**
   * Execute the function with streaming results.
   *
   * This function:
   * 1. Gets an authenticated SDK client (or anonymous client for non-logged-in users)
   * 2. Builds execution options including streaming, caching, and optional reasoning
   * 3. Calls Functions.Executions.create with the selected profile
   * 4. Processes streaming chunks, merging completions and tasks progressively
   * 5. Updates UI state as chunks arrive for real-time feedback
   *
   * Chunk merging strategy:
   * - `output`: Takes the latest value (replaced on each chunk)
   * - `tasks`: Merged by index, with completions content concatenated
   * - `usage`: Takes the latest value
   * - `reasoning`: Concatenates content across chunks
   *
   * Error handling: Catches execution errors and displays them in the UI.
   * The user can retry by clicking Execute again.
   */
  const handleRun = async () => {
    const selectedProfile = availableProfiles[selectedProfileIndex];
    if (!functionDetails || !selectedProfile) return;

    setIsRunning(true);
    setRunError(null);
    setResults(null);
    setSplitItems(null);
    setShowAllModels(false);
    setExpandedVotes(new Set());

    try {
      // Get authenticated client (or public client for anonymous users)
      const client = await getClient();

      // Build execution options with streaming and optional reasoning
      const executionBody = {
        // Type assertion needed: local InputValue type is compatible with SDK's InputValue but TS can't verify
        input: formData as unknown as Parameters<typeof Functions.Executions.create>[3]["input"],
        stream: true as const,
        from_cache: DEV_EXECUTION_OPTIONS.from_cache,
        from_rng: demoMode,
        reasoning: reasoningEnabled ? {
          model: {
            model: reasoningModel,
            output_mode: "instruction" as const,
          },
        } : undefined,
      };

      // Execute using SDK with streaming
      const stream = await Functions.Executions.create(
        client,
        {
          remote: "github",
          owner: functionDetails.owner,
          repository: functionDetails.repository,
          commit: functionDetails.commit,
        },
        {
          remote: "github",
          owner: selectedProfile.owner,
          repository: selectedProfile.repository,
          commit: selectedProfile.commit,
        },
        executionBody
      );

      // Accumulated state for merging chunks
      type AccumulatedTask = NonNullable<typeof results>["tasks"] extends (infer T)[] | undefined ? T : never;
      let accumulatedOutput: number | number[] | undefined;
      let accumulatedTasks: AccumulatedTask[] = [];
      let accumulatedUsage: NonNullable<typeof results>["usage"] | undefined;
      let accumulatedReasoningContent = "";

      // Helper: Merge completions by model, accumulating delta content
      type CompletionType = NonNullable<AccumulatedTask["completions"]>[number];
      const mergeCompletions = (existing: CompletionType[] | undefined, incoming: CompletionType[] | undefined): CompletionType[] | undefined => {
        if (!Array.isArray(incoming) || incoming.length === 0) return existing;
        if (!Array.isArray(existing) || existing.length === 0) return incoming;

        const result = [...existing];
        for (const comp of incoming) {
          if (!comp) continue;
          const existingIdx = result.findIndex(c => c.model === comp.model);
          if (existingIdx === -1) {
            result.push(comp);
          } else {
            const existingComp = result[existingIdx];
            const existingContent = existingComp.choices?.[0]?.delta?.content || existingComp.choices?.[0]?.message?.content || "";
            const incomingContent = comp.choices?.[0]?.delta?.content || "";
            const mergedContent = existingContent + incomingContent;

            result[existingIdx] = {
              ...existingComp,
              choices: [{
                ...existingComp.choices?.[0],
                delta: { content: mergedContent },
                message: comp.choices?.[0]?.message || existingComp.choices?.[0]?.message,
              }],
            };
          }
        }
        return result;
      };

      // Helper: Merge tasks by index
      const mergeTasks = (existing: AccumulatedTask[], incoming: AccumulatedTask[]): AccumulatedTask[] => {
        const result = [...existing];
        for (const task of incoming) {
          if (!task) continue;
          const taskIndex = (task as { index?: number }).index;
          const existingIdx = result.findIndex(t => t && (t as { index?: number }).index === taskIndex);
          if (existingIdx === -1) {
            result.push(task);
          } else {
            const existingTask = result[existingIdx];
            result[existingIdx] = {
              ...existingTask,
              votes: Array.isArray(task.votes) && task.votes.length > 0 ? task.votes : existingTask?.votes,
              completions: mergeCompletions(existingTask?.completions, task.completions),
              scores: Array.isArray(task.scores) && task.scores.length > 0 ? task.scores : existingTask?.scores,
            };
          }
        }
        return result;
      };

      // Stream chunks and update UI progressively
      for await (const chunk of stream) {
        // Check for errors in chunk
        if (chunk.error) {
          throw new Error(typeof chunk.error === 'object' ? JSON.stringify(chunk.error) : String(chunk.error));
        }

        // Merge output (take latest)
        if (chunk.output !== undefined) {
          accumulatedOutput = chunk.output as number | number[];
        }

        // Merge tasks
        if (chunk.tasks && Array.isArray(chunk.tasks)) {
          accumulatedTasks = mergeTasks(accumulatedTasks, chunk.tasks as AccumulatedTask[]);
        }

        // Merge usage (take latest)
        if (chunk.usage) {
          accumulatedUsage = chunk.usage as NonNullable<typeof results>["usage"];
        }

        // Merge reasoning content
        const reasoningChunk = chunk.reasoning as { choices?: Array<{ delta?: { content?: string }; message?: { content?: string } }> } | undefined;
        if (reasoningChunk?.choices?.[0]?.delta?.content) {
          accumulatedReasoningContent += reasoningChunk.choices[0].delta.content;
        } else if (reasoningChunk?.choices?.[0]?.message?.content) {
          accumulatedReasoningContent = reasoningChunk.choices[0].message.content;
        }

        // Update UI progressively
        setResults({
          output: accumulatedOutput,
          inputSnapshot: typeof formData === 'object' && formData !== null ? { ...(formData as Record<string, unknown>) } : {},
          usage: accumulatedUsage,
          tasks: accumulatedTasks.length > 0 ? accumulatedTasks : undefined,
          reasoning: accumulatedReasoningContent ? {
            choices: [{ message: { content: accumulatedReasoningContent } }]
          } : undefined,
        });
      }
    } catch (err) {
      if (err instanceof ObjectiveAIFetchError) {
        const code = err.code;
        if (code === 401 || code === 403) {
          setRunError("Authentication required. Please sign in to execute functions.");
        } else if (code === 429) {
          setRunError("Rate limit exceeded. Please try again later.");
        } else {
          setRunError(err.message || `API error (${code})`);
        }
      } else {
        setRunError(err instanceof Error ? err.message : "Execution failed");
      }
    } finally {
      setIsRunning(false);
    }
  };

  // Build input fields — schema-driven when available, freeform otherwise
  const renderInputFields = () => {
    if (functionDetails?.inputSchema) {
      // Use SchemaFormBuilder for functions with schemas - supports typed fields (image, audio, video, file)
      return (
        <SchemaFormBuilder
          schema={functionDetails.inputSchema as unknown as InputSchema}
          value={formData}
          onChange={setFormData}
          disabled={isRunning}
        />
      );
    }

    // Use InputBuilder for freeform input (no schema)
    return (
      <InputBuilder
        value={formData}
        onChange={setFormData}
        disabled={isRunning}
        label="Input"
        description="Build your input data"
      />
    );
  };

  // Score color gradient: green (100%) → yellow (66%) → orange (33%) → red (0%)
  const getScoreColor = (percentage: number): string => {
    if (percentage >= 66) return "var(--color-success)"; // green
    if (percentage >= 33) return "var(--color-warning)"; // yellow
    if (percentage >= 15) return "var(--color-danger)";  // orange
    return "var(--color-error)";                          // red
  };

  // Helper to get content item label
  const getContentLabel = (index: number): string => {
    const letters = ["A", "B", "C", "D", "E", "F", "G", "H"];
    const input = results?.inputSnapshot;

    // Try to get actual content from input
    const contentItems = input?.contentItems as unknown[] | undefined;
    if (contentItems && contentItems[index] !== undefined) {
      const item = contentItems[index];
      if (typeof item === "string") {
        // RichContent::Text - plain string
        return item.length > 40 ? item.slice(0, 40) + "..." : item;
      }
      // RichContent::Parts - array of RichContentPart
      // See: objectiveai-rs/src/chat/completions/request/message.rs
      if (Array.isArray(item) && item.length > 0) {
        const part = item[0] as { type?: string; file?: { filename?: string } };
        if (part?.type === "file" && part?.file?.filename) {
          return part.file.filename;
        }
        if (part?.type === "image_url") return "[Image]";
        if (part?.type === "input_audio") return "[Audio]";
        if (part?.type === "video_url") return "[Video]";
        return "[Media content]";
      }
    }

    return `Option ${letters[index] || index + 1}`;
  };

  // Render results based on output type
  const renderResults = () => {
    if (!results?.output) return null;

    const output = results.output;

    // Scalar output (single number)
    if (typeof output === "number") {
      const pct = output * 100;
      const keywords = results.inputSnapshot?.keywords as string[] | undefined;
      const scoreColor = getScoreColor(pct);

      return (
        <div>
          <p style={{
            fontSize: "13px",
            color: "var(--text-muted)",
            marginBottom: "6px",
          }}>
            Overall Score
          </p>
          <p style={{
            fontSize: isMobile ? "42px" : "56px",
            fontWeight: 700,
            color: scoreColor,
            lineHeight: 1,
            marginBottom: "12px",
          }}>
            {pct.toFixed(1)}%
          </p>
          {/* Score bar */}
          <div style={{
            height: "10px",
            background: "var(--border)",
            borderRadius: "5px",
            overflow: "hidden",
            marginBottom: "16px",
          }}>
            <div style={{
              height: "100%",
              width: `${pct}%`,
              background: scoreColor,
              borderRadius: "5px",
              transition: "width 0.5s ease",
            }} />
          </div>
          {keywords && keywords.length > 0 && (
            <p style={{ fontSize: "13px", color: "var(--text-muted)" }}>
              Relevance to: <span style={{ color: "var(--text)" }}>{keywords.join(", ")}</span>
            </p>
          )}
        </div>
      );
    }

    // Vector output (array of numbers) - Rankings
    if (Array.isArray(output)) {
      const sorted = output
        .map((score, i) => ({ index: i, score, label: getContentLabel(i) }))
        .sort((a, b) => b.score - a.score);

      const keywords = results.inputSnapshot?.keywords as string[] | undefined;

      // Determine display mode based on split items
      const displayMode = splitItems ? getDisplayMode(splitItems) : "simple";
      const showCompactDisplay = displayMode === "simple" || displayMode === "mixed";

      return (
        <div>
          {/* Show keywords context */}
          {keywords && keywords.length > 0 && (
            <p style={{
              fontSize: "13px",
              color: "var(--text-muted)",
              marginBottom: "16px",
            }}>
              Ranked by relevance to: <span style={{ color: "var(--text)" }}>{keywords.join(", ")}</span>
            </p>
          )}

          <p style={{
            fontSize: "13px",
            color: "var(--text-muted)",
            marginBottom: "12px",
          }}>
            Rankings
          </p>

          <div style={{ display: "flex", flexDirection: "column", gap: isMobile ? "6px" : "8px" }}>
            {sorted.map((item, rank) => {
              const pct = item.score * 100;
              const isTop = rank === 0;
              const splitItem = splitItems?.[item.index];

              return (
                <div key={item.index} style={{
                  display: "flex",
                  alignItems: showCompactDisplay ? "center" : "flex-start",
                  gap: isMobile ? "10px" : "14px",
                  padding: isMobile ? "10px 12px" : "14px 18px",
                  background: isTop ? "rgba(34, 197, 94, 0.08)" : "var(--page-bg)",
                  borderRadius: isMobile ? "10px" : "14px",
                  border: isTop ? "1px solid rgba(34, 197, 94, 0.2)" : "1px solid transparent",
                }}>
                  <span style={{
                    fontSize: isMobile ? "14px" : "16px",
                    fontWeight: 700,
                    color: getScoreColor(pct),
                    width: isMobile ? "42px" : "50px",
                    flexShrink: 0,
                  }}>
                    {pct.toFixed(0)}%
                  </span>
                  <div style={{
                    flex: 1,
                    fontSize: isMobile ? "13px" : "14px",
                    fontWeight: isTop ? 600 : 400,
                    color: isTop ? "var(--text)" : "var(--text-muted)",
                    overflow: "hidden",
                    minWidth: 0,
                  }}>
                    {splitItem !== undefined ? (
                      <SplitItemDisplay
                        item={toDisplayItem(splitItem)}
                        compact={showCompactDisplay}
                      />
                    ) : (
                      <span style={{
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        display: "block",
                      }}>
                        {item.label}
                      </span>
                    )}
                  </div>
                  {isTop && !isMobile && (
                    <span style={{
                      fontSize: "11px",
                      padding: "3px 8px",
                      background: "rgba(34, 197, 94, 0.15)",
                      color: "var(--color-success)",
                      borderRadius: "6px",
                      fontWeight: 600,
                      flexShrink: 0,
                    }}>
                      Best Match
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      );
    }

    return null;
  };

  // Loading state
  if (isLoadingDetails) {
    return <SkeletonFunctionDetails />;
  }

  // Error state
  if (loadError || !functionDetails) {
    return (
      <div className="page">
        <div className="container" style={{ paddingTop: "100px", textAlign: "center" }}>
          <p style={{ color: "var(--color-error)", marginBottom: "8px" }}>Failed to load function</p>
          <p style={{ color: "var(--text-muted)", marginBottom: "24px" }}>{loadError}</p>
          <Link href="/functions" style={{ color: "var(--accent)" }}>
            Back to Functions
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="page">
      <div className="container">
        {/* Breadcrumb Row with Pin */}
        <div style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "baseline",
          marginBottom: "20px",
          fontSize: "14px",
        }}>
          <nav style={{
            display: "flex",
            gap: "8px",
            color: "var(--text-muted)",
            flexWrap: "wrap",
          }}>
            <Link href="/functions" style={{ color: "var(--accent)", textDecoration: "none" }}>
              Functions
            </Link>
            <span>/</span>
            <span>{functionDetails.name}</span>
          </nav>
          <button
            onClick={toggleSave}
            style={{
              background: "none",
              border: "none",
              padding: 0,
              cursor: "pointer",
              fontSize: "inherit",
              color: showPinnedColor ? "var(--accent)" : "var(--text-muted)",
              opacity: 0.7,
              transition: showPinnedColor ? "color 0.15s ease-in" : "color 0.5s ease-out",
            }}
          >
            {isSaved ? "Pinned Function" : "Pin Function"}
          </button>
        </div>

        {/* Header */}
        <div style={{ marginBottom: isMobile ? "20px" : "28px" }}>
          <h1 className="heading2" style={{ marginBottom: "4px" }}>
            {functionDetails.name}
          </h1>
          <p style={{
            fontSize: isMobile ? "15px" : "17px",
            color: "var(--text-muted)",
            maxWidth: "700px",
            lineHeight: 1.6,
            marginBottom: "8px",
          }}>
            {functionDetails.description}
          </p>
          <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
            <span className="tag" style={{ display: "inline-block" }}>{functionDetails.category}</span>
            <span style={{
              fontSize: "12px",
              padding: "4px 12px",
              background: "var(--border)",
              borderRadius: "12px",
              color: "var(--text-muted)",
            }}>
              {functionDetails.owner}/{functionDetails.repository}
            </span>
          </div>
        </div>

        {/* Main Layout */}
        <div style={{
          display: isMobile ? "flex" : "grid",
          flexDirection: "column",
          gridTemplateColumns: "1fr 1fr",
          gap: isMobile ? "16px" : "32px",
          alignItems: isMobile ? "stretch" : "start",
          maxWidth: "900px",
        }}>
          {/* Left - Input */}
          <div className="card" style={{ padding: isMobile ? "16px" : undefined }}>
            <h3 style={{
              fontSize: isMobile ? "11px" : "12px",
              fontWeight: 600,
              marginBottom: isMobile ? "16px" : "24px",
              textTransform: "uppercase",
              letterSpacing: "0.05em",
              color: "var(--text-muted)",
            }}>
              Input
            </h3>

            <div style={{ display: "flex", flexDirection: "column", gap: isMobile ? "16px" : "24px" }}>
              {renderInputFields()}
            </div>

            <div style={{ marginTop: isMobile ? "16px" : "24px" }}>
              <label style={{
                display: "block",
                fontSize: "14px",
                fontWeight: 600,
                marginBottom: "8px",
                color: "var(--text)",
              }}>
                Profile
                <span style={{
                  fontWeight: 400,
                  color: "var(--text-muted)",
                  marginLeft: "8px",
                }}>
                  Learned weights for this function
                </span>
              </label>
              <select
                className="select"
                value={selectedProfileIndex}
                onChange={(e) => setSelectedProfileIndex(parseInt(e.target.value, 10))}
                style={{
                  width: "100%",
                  padding: isMobile ? "10px 12px" : "12px 16px",
                  fontSize: isMobile ? "14px" : "15px",
                  background: "var(--page-bg)",
                  border: "1px solid var(--border)",
                  borderRadius: "8px",
                  color: "var(--text)",
                  cursor: "pointer",
                }}
              >
                {availableProfiles.map((profile, idx) => (
                  <option key={`${profile.owner}/${profile.repository}`} value={idx}>
                    {profile.label} — {profile.description}
                  </option>
                ))}
              </select>
            </div>

            {/* Reasoning Options */}
            <div style={{
              marginTop: isMobile ? "16px" : "24px",
              padding: isMobile ? "12px" : "16px",
              background: "var(--page-bg)",
              borderRadius: isMobile ? "10px" : "12px",
              border: "1px solid var(--border)",
            }}>
              <label style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                cursor: "pointer",
              }}>
                <input
                  type="checkbox"
                  checked={reasoningEnabled}
                  onChange={(e) => setReasoningEnabled(e.target.checked)}
                  style={{
                    width: "18px",
                    height: "18px",
                    accentColor: "var(--accent)",
                    cursor: "pointer",
                  }}
                />
                <span style={{
                  fontSize: "14px",
                  fontWeight: 600,
                  color: "var(--text)",
                }}>
                  Enable Reasoning
                </span>
              </label>

              {reasoningEnabled && (
                <div style={{ marginTop: "12px" }}>
                  <select
                    className="select"
                    value={reasoningModel}
                    onChange={(e) => setReasoningModel(e.target.value)}
                    style={{
                      width: "100%",
                      padding: isMobile ? "10px 12px" : "12px 16px",
                      fontSize: isMobile ? "14px" : "15px",
                    }}
                  >
                    {reasoningModels.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </div>
              )}

              <p style={{
                fontSize: "12px",
                color: "var(--text-muted)",
                marginTop: "10px",
                lineHeight: 1.4,
              }}>
                {reasoningEnabled
                  ? "AI will explain the result. Cost: ~$0.0001-0.001 per execution."
                  : "Generate an AI explanation of the result."}
              </p>
            </div>

            {/* Demo Mode Toggle */}
            <div style={{
              marginTop: isMobile ? "16px" : "20px",
              padding: "12px 16px",
              background: "var(--bg-secondary)",
              borderRadius: "8px",
              border: "1px solid var(--border)",
            }}>
              <label style={{
                display: "flex",
                alignItems: "center",
                gap: "10px",
                cursor: "pointer",
              }}>
                <input
                  type="checkbox"
                  checked={demoMode}
                  onChange={(e) => setDemoMode(e.target.checked)}
                  style={{
                    width: "18px",
                    height: "18px",
                    accentColor: "var(--accent)",
                    cursor: "pointer",
                  }}
                />
                <span style={{
                  fontSize: "14px",
                  fontWeight: 600,
                  color: "var(--text)",
                }}>
                  Demo Mode
                </span>
              </label>
              <p style={{
                fontSize: "12px",
                color: "var(--text-muted)",
                marginTop: "8px",
                lineHeight: 1.4,
              }}>
                {demoMode
                  ? "Simulated results (free, instant). Scores are generated via RNG."
                  : "Real LLM inference. Costs credits per execution."}
              </p>
            </div>

            <button
              className="pillBtn"
              onClick={handleRun}
              disabled={isRunning}
              style={{
                width: "100%",
                marginTop: isMobile ? "20px" : "32px",
                padding: isMobile ? "12px 16px" : undefined,
                opacity: isRunning ? 0.7 : 1,
              }}
            >
              {isRunning ? "Running..." : "Execute"}
            </button>
          </div>

          {/* Right - Results */}
          <div className="card" style={{ padding: isMobile ? "16px" : undefined }}>
            <h3 style={{
              fontSize: isMobile ? "11px" : "12px",
              fontWeight: 600,
              marginBottom: isMobile ? "16px" : "24px",
              textTransform: "uppercase",
              letterSpacing: "0.05em",
              color: "var(--text-muted)",
            }}>
              Output
            </h3>

            {!results && !isRunning && !runError && (
              <div style={{
                textAlign: "center",
                padding: isMobile ? "40px 16px" : "60px 20px",
                color: "var(--text-muted)",
              }}>
                <p style={{ marginBottom: "8px", fontSize: "24px" }}>—</p>
                <p style={{ fontSize: "14px" }}>Run the function to see results</p>
              </div>
            )}

            {isRunning && (
              <div style={{
                textAlign: "center",
                padding: isMobile ? "40px 16px" : "60px 20px",
                color: "var(--text-muted)",
              }}>
                <div style={{
                  width: "40px",
                  height: "40px",
                  border: "3px solid var(--border)",
                  borderTopColor: "var(--accent)",
                  borderRadius: "50%",
                  margin: "0 auto 16px",
                  animation: "spin 1s linear infinite",
                }} />
                <p style={{ fontSize: "14px" }}>Processing...</p>
              </div>
            )}

            {runError && !isRunning && !results && (
              <div style={{
                textAlign: "center",
                padding: isMobile ? "40px 16px" : "60px 20px",
              }}>
                <p style={{ color: "var(--color-error)", marginBottom: "8px" }}>
                  {runError.includes("401") ? "Not authenticated" : "Execution failed"}
                </p>
                <p style={{ fontSize: "14px", color: "var(--text-muted)" }}>
                  {runError.includes("401")
                    ? "API key missing or invalid. Contact admin to set up access."
                    : runError}
                </p>
              </div>
            )}

            {results && !isRunning && (
              <div style={{ display: "flex", flexDirection: "column", gap: isMobile ? "16px" : "24px" }}>
                {renderResults()}

                {/* Model Breakdown - minimal style matching mockup */}
                {results.tasks && Array.isArray(results.tasks) && results.tasks.length > 0 && results.tasks[0]?.votes && results.tasks[0].votes.length > 0 && (
                  <div>
                    {(() => {
                      const votes = results.tasks![0].votes!;
                      const allSimulated = votes.every(v => v.from_rng);
                      const letters = ["A", "B", "C", "D", "E", "F", "G", "H"];

                      // Get content labels from split items or fallback to input
                      const getOptionLabel = (idx: number): string => {
                        // Use split items if available (simplified, actual content)
                        if (splitItems && splitItems[idx] !== undefined) {
                          const item = splitItems[idx];
                          if (typeof item === "string") {
                            return item.length > 18 ? item.slice(0, 18) + "…" : item;
                          }
                          if (typeof item === "number" || typeof item === "boolean") {
                            return String(item);
                          }
                          // For complex items, show a brief summary
                          const display = toDisplayItem(item);
                          if (display.type === "image") return "[Image]";
                          if (display.type === "audio") return "[Audio]";
                          if (display.type === "video") return "[Video]";
                          if (display.type === "file") return display.filename || "[File]";
                          if (display.type === "object" || display.type === "array") {
                            const json = JSON.stringify(item);
                            return json.length > 18 ? json.slice(0, 18) + "…" : json;
                          }
                          return String(item);
                        }
                        // Fallback to old behavior
                        const contentItems = results.inputSnapshot?.contentItems as unknown[] | undefined;
                        if (contentItems && contentItems[idx]) {
                          const item = contentItems[idx];
                          if (typeof item === "string") {
                            return item.length > 18 ? item.slice(0, 18) + "…" : item;
                          }
                        }
                        return `Option ${letters[idx] || idx + 1}`;
                      };

                      return (
                        <>
                          <p style={{
                            fontSize: isMobile ? "12px" : "13px",
                            color: "var(--text-muted)",
                            marginBottom: isMobile ? "12px" : "16px",
                          }}>
                            Model Breakdown
                          </p>

                          <div className="model-breakdown-wrapper">
                            <div style={{ display: "flex", flexDirection: "column", gap: isMobile ? "12px" : "16px" }}>
                            {(() => {
                              const displayedVotes = showAllModels ? votes : votes.slice(0, 5);
                              const completions = results.tasks?.[0]?.completions || [];

                              return displayedVotes.map((vote, modelIdx) => {
                                const maxVoteIdx = vote.vote.indexOf(Math.max(...vote.vote));
                                const confidence = Math.max(...vote.vote) * 100;
                                // Use readable model name if available, else shortened cryptic ID
                                const displayName = modelNames[vote.model] || vote.model.slice(0, 8);
                                const isResolved = !!modelNames[vote.model];
                                const isExpanded = expandedVotes.has(modelIdx);
                                // Find matching completion by model ID
                                const completion = completions.find(c => c.model === vote.model);
                                // Handle both streaming (delta) and non-streaming (message) structures
                                const choice = completion?.choices?.[0];
                                const reasoningText = choice?.message?.content || choice?.delta?.content;

                                return (
                                  <div key={modelIdx}>
                                    <div
                                      style={{
                                        display: "flex",
                                        justifyContent: "space-between",
                                        alignItems: isMobile ? "flex-start" : "baseline",
                                        flexDirection: isMobile ? "column" : "row",
                                        gap: isMobile ? "4px" : "0",
                                        marginBottom: "8px",
                                        cursor: reasoningText ? "pointer" : "default",
                                      }}
                                      onClick={() => {
                                        if (!reasoningText) return;
                                        setExpandedVotes(prev => {
                                          const next = new Set(prev);
                                          if (next.has(modelIdx)) {
                                            next.delete(modelIdx);
                                          } else {
                                            next.add(modelIdx);
                                          }
                                          return next;
                                        });
                                      }}
                                    >
                                      <span style={{ fontSize: isMobile ? "12px" : "13px", color: "var(--text)" }}>
                                        {reasoningText && (
                                          <span style={{
                                            display: "inline-block",
                                            width: "16px",
                                            color: "var(--text-muted)",
                                            fontSize: "10px",
                                          }}>
                                            {isExpanded ? "▼" : "▶"}
                                          </span>
                                        )}
                                        <span
                                          className={isResolved ? "model-name" : "model-id"}
                                          style={{
                                            fontFamily: isResolved ? "inherit" : "monospace",
                                            fontSize: isResolved ? (isMobile ? "12px" : "13px") : (isMobile ? "11px" : "12px"),
                                            color: isResolved ? "var(--text)" : "var(--text-muted)",
                                          }}
                                        >
                                          {displayName}
                                        </span>
                                        <span style={{ margin: "0 6px", color: "var(--text-muted)" }}>→</span>
                                        {isMobile ? getOptionLabel(maxVoteIdx).slice(0, 15) + (getOptionLabel(maxVoteIdx).length > 15 ? "…" : "") : getOptionLabel(maxVoteIdx)}
                                      </span>
                                      <span style={{ fontSize: isMobile ? "12px" : "13px" }}>
                                        <span style={{ color: getScoreColor(confidence), fontWeight: 500 }}>
                                          {confidence.toFixed(0)}%
                                        </span>
                                        {!isMobile && (
                                          <span style={{ color: "var(--text-muted)", marginLeft: "8px", fontSize: "11px" }}>
                                            w:{vote.weight}
                                          </span>
                                        )}
                                      </span>
                                    </div>
                                    {/* Progress bar - muted fill, no color */}
                                    <div style={{
                                      height: "6px",
                                      background: "var(--border)",
                                      borderRadius: "3px",
                                      overflow: "hidden",
                                    }}>
                                      <div style={{
                                        height: "100%",
                                        width: `${confidence}%`,
                                        background: "var(--text-muted)",
                                        borderRadius: "3px",
                                        opacity: 0.4,
                                      }} />
                                    </div>
                                    {/* Expanded reasoning */}
                                    {isExpanded && reasoningText && (
                                      <div style={{
                                        marginTop: "8px",
                                        padding: "12px",
                                        background: "var(--page-bg)",
                                        borderRadius: "8px",
                                        fontSize: "12px",
                                        color: "var(--text-muted)",
                                        lineHeight: 1.5,
                                        whiteSpace: "pre-wrap",
                                      }}>
                                        {reasoningText}
                                      </div>
                                    )}
                                  </div>
                                );
                              });
                            })()}
                            {votes.length > 5 && (
                              <button
                                onClick={() => setShowAllModels(!showAllModels)}
                                style={{
                                  background: "none",
                                  border: "none",
                                  padding: 0,
                                  fontSize: "12px",
                                  color: "var(--accent)",
                                  cursor: "pointer",
                                  textAlign: "left",
                                }}
                              >
                                {showAllModels
                                  ? "Show less"
                                  : `+${votes.length - 5} more model${votes.length - 5 !== 1 ? "s" : ""}`
                                }
                              </button>
                            )}
                            </div>
                          </div>

                          {allSimulated && (
                            <p style={{
                              marginTop: "16px",
                              fontSize: "11px",
                              color: "var(--text-muted)",
                              opacity: 0.7,
                            }}>
                              Demo mode — results simulated
                            </p>
                          )}
                        </>
                      );
                    })()}
                  </div>
                )}

                {/* Reasoning Summary */}
                {results.reasoning?.choices?.[0]?.message?.content && (
                  <div style={{
                    padding: isMobile ? "12px" : "16px",
                    background: "var(--page-bg)",
                    borderRadius: isMobile ? "10px" : "12px",
                    border: "1px solid var(--border)",
                  }}>
                    <p style={{
                      fontSize: isMobile ? "12px" : "13px",
                      color: "var(--text-muted)",
                      marginBottom: isMobile ? "8px" : "12px",
                    }}>
                      Reasoning Summary
                    </p>
                    <p style={{
                      fontSize: isMobile ? "13px" : "14px",
                      color: "var(--text)",
                      lineHeight: 1.6,
                      whiteSpace: "pre-wrap",
                    }}>
                      {results.reasoning.choices[0].message.content}
                    </p>
                  </div>
                )}

                {/* Usage & Cost */}
                {results.usage && (
                  <div style={{
                    padding: isMobile ? "10px 12px" : "12px 16px",
                    background: "var(--page-bg)",
                    borderRadius: isMobile ? "10px" : "12px",
                    fontSize: isMobile ? "12px" : "13px",
                    color: "var(--text-muted)",
                    display: "flex",
                    flexWrap: "wrap",
                    gap: isMobile ? "12px" : "16px",
                  }}>
                    <span>
                      {results.usage.total_tokens.toLocaleString()} tokens
                    </span>
                    {results.usage.cost !== undefined && (
                      <span style={{ color: "var(--text)" }}>
                        ${results.usage.cost.toFixed(4)}
                      </span>
                    )}
                    {!isMobile && results.usage.total_cost !== undefined && results.usage.total_cost !== results.usage.cost && (
                      <span>
                        (${results.usage.total_cost.toFixed(4)} total)
                      </span>
                    )}
                  </div>
                )}

              </div>
            )}
          </div>
        </div>

      </div>
    </div>
  );
}
