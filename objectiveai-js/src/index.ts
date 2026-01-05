import OpenAI from "openai";
import { Stream } from "openai/streaming";

// Message

export type Message =
  | Message.Developer
  | Message.System
  | Message.User
  | Message.Tool
  | Message.Assistant;

export namespace Message {
  export type SimpleContent = string | SimpleContentPart[];

  export interface SimpleContentPart {
    type: "text";
    text: string;
  }

  export type RichContent = string | RichContentPart[];

  export type RichContentPart =
    | RichContentPart.Text
    | RichContentPart.ImageUrl
    | RichContentPart.InputAudio
    | RichContentPart.InputVideo
    | RichContentPart.VideoUrl
    | RichContentPart.File;

  export namespace RichContentPart {
    export interface Text {
      type: "text";
      text: string;
    }

    export interface ImageUrl {
      type: "image_url";
      image_url: ImageUrl.Definition;
    }

    export namespace ImageUrl {
      export interface Definition {
        url: string;
        detail?: Detail | null;
      }

      export type Detail = "auto" | "low" | "high";
    }

    export interface InputAudio {
      type: "input_audio";
      input_audio: InputAudio.Definition;
    }

    export namespace InputAudio {
      export interface Definition {
        data: string;
        format: Format;
      }

      export type Format = "wav" | "mp3";
    }

    export interface InputVideo {
      type: "input_video";
      video_url: InputVideo.Definition;
    }

    export namespace InputVideo {
      export interface Definition {
        url: string;
      }
    }

    export interface VideoUrl {
      type: "video_url";
      video_url: InputVideo.Definition;
    }

    export interface File {
      type: "file";
      file: File.Definition;
    }

    export namespace File {
      export interface Definition {
        file_data?: string | null;
        file_id?: string | null;
        filename?: string | null;
        file_url?: string | null;
      }
    }
  }

  export interface Developer {
    role: "developer";
    content: SimpleContent;
    name?: string;
  }

  export interface System {
    role: "system";
    content: SimpleContent;
    name?: string;
  }

  export interface User {
    role: "user";
    content: RichContent;
    name?: string;
  }

  export interface Tool {
    role: "tool";
    content: RichContent;
    tool_call_id: string;
  }

  export interface Assistant {
    role: "assistant";
    content?: RichContent | null;
    name?: string | null;
    refusal?: string | null;
    tool_calls?: Assistant.ToolCall[] | null;
    reasoning?: string | null;
  }

  export namespace Assistant {
    export type ToolCall = ToolCall.Function;

    export namespace ToolCall {
      export interface Function {
        type: "function";
        id: string;
        function: Function.Definition;
      }

      export namespace Function {
        export interface Definition {
          name: string;
          arguments: string;
        }
      }
    }
  }
}

// Ensemble LLM

export interface EnsembleLlmBase {
  model: string;
  output_mode: EnsembleLlm.OutputMode;
  synthetic_reasoning?: boolean | null;
  top_logprobs?: number | null;
  prefix_messages?: Message[] | null;
  suffix_messages?: Message[] | null;
  frequency_penalty?: number | null;
  logit_bias?: Record<string, number> | null;
  max_completion_tokens?: number | null;
  presence_penalty?: number | null;
  stop?: EnsembleLlm.Stop | null;
  temperature?: number | null;
  top_p?: number | null;
  max_tokens?: number | null;
  min_p?: number | null;
  provider?: EnsembleLlm.Provider | null;
  reasoning?: EnsembleLlm.Reasoning | null;
  repetition_penalty?: number | null;
  top_a?: number | null;
  top_k?: number | null;
  verbosity?: EnsembleLlm.Verbosity | null;
}

export interface EnsembleLlmBaseWithFallbacksAndCount {
  count?: number | null;
  inner: EnsembleLlmBase;
  fallbacks?: EnsembleLlmBase[] | null;
}

export interface EnsembleLlm extends EnsembleLlmBase {
  id: string;
}

export interface EnsembleLlmWithFallbacksAndCount {
  count?: number | null;
  inner: EnsembleLlm;
  fallbacks?: EnsembleLlm[] | null;
}

export namespace EnsembleLlm {
  export type OutputMode = "instruction" | "json_schema" | "tool_call";

  export type Stop = string | string[];

  export interface Provider {
    allow_fallbacks?: boolean | null;
    require_parameters?: boolean | null;
    order?: string[] | null;
    only?: string[] | null;
    ignore?: string[] | null;
    quantizations?: Provider.Quantization[] | null;
  }

  export namespace Provider {
    export type Quantization =
      | "int4"
      | "int8"
      | "fp4"
      | "fp6"
      | "fp8"
      | "fp16"
      | "bf16"
      | "fp32"
      | "unknown";
  }

  export interface Reasoning {
    enabled?: boolean | null;
    max_tokens?: number | null;
    effort?: Reasoning.Effort | null;
    summary_verbosity?: Reasoning.SummaryVerbosity | null;
  }

  export namespace Reasoning {
    export type Effort =
      | "none"
      | "minimal"
      | "low"
      | "medium"
      | "high"
      | "xhigh";

    export type SummaryVerbosity = "auto" | "concise" | "detailed";
  }

  export type Verbosity = "low" | "medium" | "high";

  export interface ListItem {
    id: string;
  }

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/ensemble_llms", options);
    return response as { data: ListItem[] };
  }

  export interface RetrieveItem extends EnsembleLlm {
    created: number;
  }

  export async function retrieve(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<RetrieveItem> {
    const response = await openai.get(`/ensemble_llms/${id}`, options);
    return response as RetrieveItem;
  }

  export interface HistoricalUsage {
    requests: number;
    completion_tokens: number;
    prompt_tokens: number;
    total_cost: number;
  }

  export async function retrieveUsage(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<HistoricalUsage> {
    const response = await openai.get(`/ensemble_llms/${id}/usage`, options);
    return response as HistoricalUsage;
  }
}

// Ensemble

export interface EnsembleBase {
  llms: EnsembleLlmBaseWithFallbacksAndCount[];
}

export interface Ensemble {
  id: string;
  llms: EnsembleLlmWithFallbacksAndCount[];
}

export namespace Ensemble {
  export interface ListItem {
    id: string;
  }

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/ensembles", options);
    return response as { data: ListItem[] };
  }

  export interface RetrieveItem extends Ensemble {
    created: number;
  }

  export async function retrieve(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<RetrieveItem> {
    const response = await openai.get(`/ensembles/${id}`, options);
    return response as RetrieveItem;
  }

  export interface HistoricalUsage {
    requests: number;
    completion_tokens: number;
    prompt_tokens: number;
    total_cost: number;
  }

  export async function retrieveUsage(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<HistoricalUsage> {
    const response = await openai.get(`/ensembles/${id}/usage`, options);
    return response as HistoricalUsage;
  }
}

// Chat Completions

export namespace Chat {
  export namespace Completions {
    export namespace Request {
      export interface ChatCompletionCreateParamsBase {
        messages: Message[];
        provider?: Provider | null;
        model: Model;
        models?: Model[] | null;
        top_logprobs?: number | null;
        response_format?: ResponseFormat | null;
        seed?: number | null;
        tool_choice?: ToolChoice | null;
        tools?: Tool[] | null;
        parallel_tool_calls?: boolean | null;
        prediction?: Prediction | null;
        backoff_max_elapsed_time?: number | null;
        first_chunk_timeout?: number | null;
        other_chunk_timeout?: number | null;
      }

      export interface ChatCompletionCreateParamsStreaming
        extends ChatCompletionCreateParamsBase {
        stream: true;
      }

      export interface ChatCompletionCreateParamsNonStreaming
        extends ChatCompletionCreateParamsBase {
        stream?: false | null;
      }

      export type ChatCompletionCreateParams =
        | ChatCompletionCreateParamsStreaming
        | ChatCompletionCreateParamsNonStreaming;

      export interface Provider {
        data_collection?: Provider.DataCollection | null;
        zdr?: boolean | null;
        sort?: Provider.Sort | null;
        max_price?: Provider.MaxPrice | null;
        preferred_min_throughput?: number | null;
        preferred_max_latency?: number | null;
        min_throughput?: number | null;
        max_latency?: number | null;
      }

      export namespace Provider {
        export type DataCollection = "allow" | "deny";

        export type Sort = "price" | "throughput" | "latency";

        export interface MaxPrice {
          prompt?: number | null;
          completion?: number | null;
          image?: number | null;
          audio?: number | null;
          request?: number | null;
        }
      }

      export type Model = string | EnsembleLlmBase;

      export type ResponseFormat =
        | ResponseFormat.Text
        | ResponseFormat.JsonObject
        | ResponseFormat.JsonSchema
        | ResponseFormat.Grammar
        | ResponseFormat.Python;

      export namespace ResponseFormat {
        export interface Text {
          type: "text";
        }

        export interface JsonObject {
          type: "json_object";
        }

        export interface JsonSchema {
          type: "json_schema";
          json_schema: JsonSchema.JsonSchema;
        }

        export namespace JsonSchema {
          export interface JsonSchema {
            name: string;
            description?: string | null;
            schema?: JsonValue;
            strict?: boolean | null;
          }
        }

        export interface Grammar {
          type: "grammar";
          grammar: string;
        }

        export interface Python {
          type: "python";
        }
      }

      export type ToolChoice =
        | "none"
        | "auto"
        | "required"
        | ToolChoice.Function;

      export namespace ToolChoice {
        export interface Function {
          type: "function";
          function: Function.Function;
        }

        export namespace Function {
          export interface Function {
            name: string;
          }
        }
      }

      export type Tool = Tool.Function;

      export namespace Tool {
        export interface Function {
          type: "function";
          function: Function.Definition;
        }

        export namespace Function {
          export interface Definition {
            name: string;
            description?: string | null;
            parameters?: { [key: string]: JsonValue } | null;
            strict?: boolean | null;
          }
        }
      }

      export interface Prediction {
        type: "content";
        content: Prediction.Content;
      }

      export namespace Prediction {
        export type Content = string | Content.Part[];

        export namespace Content {
          export interface Part {
            type: "text";
            text: string;
          }
        }
      }
    }

    export namespace Response {
      export namespace Streaming {
        export interface ChatCompletionChunk {
          id: string;
          upstream_id: string;
          choices: Choice[];
          created: number;
          model: string;
          upstream_model: string;
          object: "chat.completion.chunk";
          service_tier?: string;
          system_fingerprint?: string;
          usage?: Usage;
          provider?: string;
        }

        export namespace ChatCompletionChunk {
          export function merged(
            a: ChatCompletionChunk,
            b: ChatCompletionChunk
          ): [ChatCompletionChunk, boolean] {
            const id = a.id;
            const upstream_id = a.upstream_id;
            const [choices, choicesChanged] = Choice.mergedList(
              a.choices,
              b.choices
            );
            const created = a.created;
            const model = a.model;
            const upstream_model = a.upstream_model;
            const object = a.object;
            const [service_tier, service_tierChanged] = merge(
              a.service_tier,
              b.service_tier
            );
            const [system_fingerprint, system_fingerprintChanged] = merge(
              a.system_fingerprint,
              b.system_fingerprint
            );
            const [usage, usageChanged] = merge(a.usage, b.usage);
            const [provider, providerChanged] = merge(a.provider, b.provider);
            if (
              choicesChanged ||
              service_tierChanged ||
              system_fingerprintChanged ||
              usageChanged ||
              providerChanged
            ) {
              return [
                {
                  id,
                  upstream_id,
                  choices,
                  created,
                  model,
                  upstream_model,
                  object,
                  ...(service_tier !== undefined ? { service_tier } : {}),
                  ...(system_fingerprint !== undefined
                    ? { system_fingerprint }
                    : {}),
                  ...(usage !== undefined ? { usage } : {}),
                  ...(provider !== undefined ? { provider } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }
        }

        export interface Choice {
          delta: Delta;
          finish_reason: FinishReason | null;
          index: number;
          logprobs?: Logprobs;
        }

        export namespace Choice {
          export function merged(a: Choice, b: Choice): [Choice, boolean] {
            const [delta, deltaChanged] = merge(a.delta, b.delta, Delta.merged);
            const [finish_reason, finish_reasonChanged] = merge(
              a.finish_reason,
              b.finish_reason
            );
            const index = a.index;
            const [logprobs, logprobsChanged] = merge(
              a.logprobs,
              b.logprobs,
              Logprobs.merged
            );
            if (deltaChanged || finish_reasonChanged || logprobsChanged) {
              return [
                {
                  delta,
                  finish_reason,
                  index,
                  ...(logprobs !== undefined ? { logprobs } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }

          export function mergedList(
            a: Choice[],
            b: Choice[]
          ): [Choice[], boolean] {
            let merged: Choice[] | undefined = undefined;
            for (const choice of b) {
              const existingIndex = a.findIndex(
                ({ index }) => index === choice.index
              );
              if (existingIndex === -1) {
                if (merged === undefined) {
                  merged = [...a, choice];
                } else {
                  merged.push(choice);
                }
              } else {
                const [mergedChoice, choiceChanged] = Choice.merged(
                  a[existingIndex],
                  choice
                );
                if (choiceChanged) {
                  if (merged === undefined) {
                    merged = [...a];
                  }
                  merged[existingIndex] = mergedChoice;
                }
              }
            }
            return merged ? [merged, true] : [a, false];
          }
        }

        export interface Delta {
          content?: string;
          refusal?: string;
          role?: Role;
          tool_calls?: ToolCall[];
          reasoning?: string;
          images?: Image[];
        }

        export namespace Delta {
          export function merged(a: Delta, b: Delta): [Delta, boolean] {
            const [content, contentChanged] = merge(
              a.content,
              b.content,
              mergedString
            );
            const [refusal, refusalChanged] = merge(
              a.refusal,
              b.refusal,
              mergedString
            );
            const [role, roleChanged] = merge(a.role, b.role);
            const [tool_calls, tool_callsChanged] = merge(
              a.tool_calls,
              b.tool_calls,
              ToolCall.mergedList
            );
            const [reasoning, reasoningChanged] = merge(
              a.reasoning,
              b.reasoning,
              mergedString
            );
            const [images, imagesChanged] = merge(
              a.images,
              b.images,
              Image.mergedList
            );
            if (
              contentChanged ||
              reasoningChanged ||
              refusalChanged ||
              roleChanged ||
              tool_callsChanged ||
              imagesChanged
            ) {
              return [
                {
                  ...(content !== undefined ? { content } : {}),
                  ...(reasoning !== undefined ? { reasoning } : {}),
                  ...(refusal !== undefined ? { refusal } : {}),
                  ...(role !== undefined ? { role } : {}),
                  ...(tool_calls !== undefined ? { tool_calls } : {}),
                  ...(images !== undefined ? { images } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }
        }

        export type ToolCall = ToolCall.Function;

        export namespace ToolCall {
          export function merged(
            a: ToolCall,
            b: ToolCall
          ): [ToolCall, boolean] {
            return Function.merged(a, b);
          }
          export function mergedList(
            a: ToolCall[],
            b: ToolCall[]
          ): [ToolCall[], boolean] {
            let merged: ToolCall[] | undefined = undefined;
            for (const toolCall of b) {
              const existingIndex = a.findIndex(
                ({ index }) => index === toolCall.index
              );
              if (existingIndex === -1) {
                if (merged === undefined) {
                  merged = [...a, toolCall];
                } else {
                  merged.push(toolCall);
                }
              } else {
                const [mergedToolCall, toolCallChanged] = ToolCall.merged(
                  a[existingIndex],
                  toolCall
                );
                if (toolCallChanged) {
                  if (merged === undefined) {
                    merged = [...a];
                  }
                  merged[existingIndex] = mergedToolCall;
                }
              }
            }
            return merged ? [merged, true] : [a, false];
          }
          export interface Function {
            index: number;
            type?: "function";
            id?: string;
            function?: Function.Definition;
          }

          export namespace Function {
            export function merged(
              a: Function,
              b: Function
            ): [Function, boolean] {
              const index = a.index;
              const [type, typeChanged] = merge(a.type, b.type);
              const [id, idChanged] = merge(a.id, b.id);
              const [function_, functionChanged] = merge(
                a.function,
                b.function,
                Definition.merged
              );
              if (idChanged || functionChanged || typeChanged) {
                return [
                  {
                    index,
                    ...(id !== undefined ? { id } : {}),
                    ...(function_ !== undefined ? { function: function_ } : {}),
                    ...(type !== undefined ? { type } : {}),
                  },
                  true,
                ];
              } else {
                return [a, false];
              }
            }
            export interface Definition {
              name?: string;
              arguments?: string;
            }

            export namespace Definition {
              export function merged(
                a: Definition,
                b: Definition
              ): [Definition, boolean] {
                const [name, nameChanged] = merge(a.name, b.name);
                const [arguments_, argumentsChanged] = merge(
                  a.arguments,
                  b.arguments,
                  mergedString
                );
                if (nameChanged || argumentsChanged) {
                  return [
                    {
                      ...(name !== undefined ? { name } : {}),
                      ...(arguments_ !== undefined
                        ? { arguments: arguments_ }
                        : {}),
                    },
                    true,
                  ];
                } else {
                  return [a, false];
                }
              }
            }
          }
        }
      }

      export namespace Unary {
        export interface ChatCompletion {
          id: string;
          upstream_id: string;
          choices: Choice[];
          created: number;
          model: string;
          upstream_model: string;
          object: "chat.completion";
          service_tier?: string;
          system_fingerprint?: string;
          usage: Usage;
          provider?: string;
        }

        export interface Choice {
          message: Message;
          finish_reason: FinishReason;
          index: number;
          logprobs: Logprobs | null;
        }

        export interface Message {
          content: string | null;
          refusal: string | null;
          role: Role;
          tool_calls: ToolCall[] | null;
          reasoning?: string;
          images?: Image[];
        }

        export type ToolCall = ToolCall.Function;

        export namespace ToolCall {
          export interface Function {
            type: "function";
            id: string;
            function: Function.Definition;
          }

          export namespace Function {
            export interface Definition {
              name: string;
              arguments: string;
            }
          }
        }
      }

      export type FinishReason =
        | "stop"
        | "length"
        | "tool_calls"
        | "content_filter"
        | "error";

      export interface Usage {
        completion_tokens: number;
        prompt_tokens: number;
        total_tokens: number;
        completion_tokens_details?: Usage.CompletionTokensDetails;
        prompt_tokens_details?: Usage.PromptTokensDetails;
        cost: number;
        cost_details?: Usage.CostDetails;
        total_cost: number;
        cost_multiplier: number;
        is_byok: boolean;
      }

      export namespace Usage {
        export interface CompletionTokensDetails {
          accepted_prediction_tokens?: number;
          audio_tokens?: number;
          reasoning_tokens?: number;
          rejected_prediction_tokens?: number;
        }

        export interface PromptTokensDetails {
          audio_tokens?: number;
          cached_tokens?: number;
          cache_write_tokens?: number;
          video_tokens?: number;
        }

        export interface CostDetails {
          upstream_inference_cost?: number;
          upstream_upstream_inference_cost?: number;
        }
      }

      export interface Logprobs {
        content: Logprobs.Logprob[] | null;
        refusal: Logprobs.Logprob[] | null;
      }

      export namespace Logprobs {
        export function merged(a: Logprobs, b: Logprobs): [Logprobs, boolean] {
          const [content, contentChanged] = merge(
            a.content,
            b.content,
            Logprob.mergedList
          );
          const [refusal, refusalChanged] = merge(
            a.refusal,
            b.refusal,
            Logprob.mergedList
          );
          if (contentChanged || refusalChanged) {
            return [{ content, refusal }, true];
          } else {
            return [a, false];
          }
        }

        export interface Logprob {
          token: string;
          bytes: number[] | null;
          logprob: number;
          top_logprobs: Logprob.TopLogprob[];
        }

        export namespace Logprob {
          export function mergedList(
            a: Logprob[],
            b: Logprob[]
          ): [Logprob[], boolean] {
            if (b.length === 0) {
              return [a, false];
            } else if (a.length === 0) {
              return [b, true];
            } else {
              return [[...a, ...b], true];
            }
          }

          export interface TopLogprob {
            token: string;
            bytes: number[] | null;
            logprob: number | null;
          }
        }
      }

      export type Role = "assistant";

      export type Image = Image.ImageUrl;

      export namespace Image {
        export function mergedList(a: Image[], b: Image[]): [Image[], boolean] {
          if (b.length === 0) {
            return [a, false];
          } else if (a.length === 0) {
            return [b, true];
          } else {
            return [[...a, ...b], true];
          }
        }
        export interface ImageUrl {
          type: "image_url";
          image_url: { url: string };
        }
      }
    }

    export async function create(
      openai: OpenAI,
      body: Request.ChatCompletionCreateParamsStreaming,
      options?: OpenAI.RequestOptions
    ): Promise<
      Stream<Response.Streaming.ChatCompletionChunk | ObjectiveAIError>
    >;
    export async function create(
      openai: OpenAI,
      body: Request.ChatCompletionCreateParamsNonStreaming,
      options?: OpenAI.RequestOptions
    ): Promise<Response.Unary.ChatCompletion>;
    export async function create(
      openai: OpenAI,
      body: Request.ChatCompletionCreateParams,
      options?: OpenAI.RequestOptions
    ): Promise<
      | Stream<Response.Streaming.ChatCompletionChunk | ObjectiveAIError>
      | Response.Unary.ChatCompletion
    > {
      const response = await openai.post("/chat/completions", {
        body,
        stream: body.stream ?? false,
        ...options,
      });
      return response as
        | Stream<Response.Streaming.ChatCompletionChunk | ObjectiveAIError>
        | Response.Unary.ChatCompletion;
    }
  }
}

// Vector Completions

export namespace Vector {
  export namespace Completions {
    export namespace Request {
      export interface VectorCompletionCreateParamsBase {
        retry?: string | null;
        messages: Message[];
        provider?: Chat.Completions.Request.Provider | null;
        ensemble: Ensemble;
        profile: number[];
        seed?: number | null;
        tools?: Chat.Completions.Request.Tool[] | null;
        responses: Message.RichContent[];
        backoff_max_elapsed_time?: number | null;
        first_chunk_timeout?: number | null;
        other_chunk_timeout?: number | null;
      }

      export interface VectorCompletionCreateParamsStreaming
        extends VectorCompletionCreateParamsBase {
        stream: true;
      }

      export interface VectorCompletionCreateParamsNonStreaming
        extends VectorCompletionCreateParamsBase {
        stream?: false | null;
      }

      export type VectorCompletionCreateParams =
        | VectorCompletionCreateParamsStreaming
        | VectorCompletionCreateParamsNonStreaming;

      export type Ensemble = string | EnsembleBase;
    }

    export namespace Response {
      export namespace Streaming {
        export interface VectorCompletionChunk {
          id: string;
          completions: ChatCompletionChunk[];
          votes: Vote[];
          scores: number[];
          weights: number[];
          created: number;
          ensemble: string;
          object: "vector.completion.chunk";
          usage?: Usage;
        }

        export namespace VectorCompletionChunk {
          export function merged(
            a: VectorCompletionChunk,
            b: VectorCompletionChunk
          ): [VectorCompletionChunk, boolean] {
            const id = a.id;
            const [completions, completionsChanged] =
              ChatCompletionChunk.mergedList(a.completions, b.completions);
            const [votes, votesChanged] = Vote.mergedList(a.votes, b.votes);
            const [scores, scoresChanged] = Scores.merged(a.scores, b.scores);
            const [weights, weightsChanged] = Weights.merged(
              a.weights,
              b.weights
            );
            const created = a.created;
            const ensemble = a.ensemble;
            const object = a.object;
            const [usage, usageChanged] = merge(a.usage, b.usage);
            if (
              completionsChanged ||
              votesChanged ||
              scoresChanged ||
              weightsChanged ||
              usageChanged
            ) {
              return [
                {
                  id,
                  completions,
                  votes,
                  scores,
                  weights,
                  created,
                  ensemble,
                  object,
                  ...(usage !== undefined ? { usage } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }
        }

        export interface ChatCompletionChunk
          extends Chat.Completions.Response.Streaming.ChatCompletionChunk {
          index: number;
          error?: ObjectiveAIError;
        }

        export namespace ChatCompletionChunk {
          export function merged(
            a: ChatCompletionChunk,
            b: ChatCompletionChunk
          ): [ChatCompletionChunk, boolean] {
            const index = a.index;
            const [base, baseChanged] =
              Chat.Completions.Response.Streaming.ChatCompletionChunk.merged(
                a,
                b
              );
            const [error, errorChanged] = merge(a.error, b.error);
            if (baseChanged || errorChanged) {
              return [
                {
                  index,
                  ...base,
                  ...(error !== undefined ? { error } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }

          export function mergedList(
            a: ChatCompletionChunk[],
            b: ChatCompletionChunk[]
          ): [ChatCompletionChunk[], boolean] {
            let merged: ChatCompletionChunk[] | undefined = undefined;
            for (const chunk of b) {
              const existingIndex = a.findIndex(
                ({ index }) => index === chunk.index
              );
              if (existingIndex === -1) {
                if (merged === undefined) {
                  merged = [...a, chunk];
                } else {
                  merged.push(chunk);
                }
              } else {
                const [mergedChunk, chunkChanged] = ChatCompletionChunk.merged(
                  a[existingIndex],
                  chunk
                );
                if (chunkChanged) {
                  if (merged === undefined) {
                    merged = [...a];
                  }
                  merged[existingIndex] = mergedChunk;
                }
              }
            }
            return merged ? [merged, true] : [a, false];
          }
        }

        export namespace Scores {
          export function merged(
            a: number[],
            b: number[]
          ): [number[], boolean] {
            if (a.length === b.length) {
              for (let i = 0; i < a.length; i++) {
                if (a[i] !== b[i]) {
                  return [b, true];
                }
              }
              return [a, false];
            } else {
              return [b, true];
            }
          }
        }

        export namespace Weights {
          export function merged(
            a: number[],
            b: number[]
          ): [number[], boolean] {
            return Scores.merged(a, b);
          }
        }
      }

      export namespace Unary {
        export interface VectorCompletion {
          id: string;
          completions: ChatCompletion[];
          votes: Vote[];
          scores: number[];
          weights: number[];
          created: number;
          ensemble: string;
          object: "vector.completion";
          usage: Usage;
        }

        export interface ChatCompletion
          extends Chat.Completions.Response.Unary.ChatCompletion {
          index: number;
          error?: ObjectiveAIError;
        }
      }

      export interface Vote {
        model: string;
        ensemble_index: number;
        flat_ensemble_index: number;
        vote: number[];
        weight: number;
        retry?: boolean;
      }

      export namespace Vote {
        export function mergedList(a: Vote[], b: Vote[]): [Vote[], boolean] {
          let merged: Vote[] | undefined = undefined;
          for (const vote of b) {
            const existingIndex = a.findIndex(
              ({ flat_ensemble_index }) =>
                flat_ensemble_index === vote.flat_ensemble_index
            );
            if (existingIndex === -1) {
              if (merged === undefined) {
                merged = [...a, vote];
              } else {
                merged.push(vote);
              }
            }
          }
          return merged ? [merged, true] : [a, false];
        }
      }

      export interface Usage {
        completion_tokens: number;
        prompt_tokens: number;
        total_tokens: number;
        completion_tokens_details?: Chat.Completions.Response.Usage.CompletionTokensDetails;
        prompt_tokens_details?: Chat.Completions.Response.Usage.PromptTokensDetails;
        cost: number;
        cost_details?: Chat.Completions.Response.Usage.CostDetails;
        total_cost: number;
      }

      export async function create(
        openai: OpenAI,
        body: Request.VectorCompletionCreateParamsStreaming,
        options?: OpenAI.RequestOptions
      ): Promise<Stream<Response.Streaming.VectorCompletionChunk>>;
      export async function create(
        openai: OpenAI,
        body: Request.VectorCompletionCreateParamsNonStreaming,
        options?: OpenAI.RequestOptions
      ): Promise<Response.Unary.VectorCompletion>;
      export async function create(
        openai: OpenAI,
        body: Request.VectorCompletionCreateParams,
        options?: OpenAI.RequestOptions
      ): Promise<
        | Stream<Response.Streaming.VectorCompletionChunk>
        | Response.Unary.VectorCompletion
      > {
        const response = await openai.post("/vector/completions", {
          body,
          stream: body.stream ?? false,
          ...options,
        });
        return response as
          | Stream<Response.Streaming.VectorCompletionChunk>
          | Response.Unary.VectorCompletion;
      }
    }
  }
}

// Function

export type Function = Function.Scalar | Function.Vector;

export namespace Function {
  export interface Scalar {
    type: "scalar.function";
    author: string;
    id: string;
    version: number;
    description: string;
    changelog?: string | null;
    input_schema: InputSchema;
    input_maps?: InputMaps | null;
    tasks: TaskExpression[];
    output: Expression;
  }

  export interface Vector {
    type: "vector.function";
    author: string;
    id: string;
    version: number;
    description: string;
    changelog?: string | null;
    input_schema: InputSchema;
    input_maps?: InputMaps | null;
    tasks: TaskExpression[];
    output: Expression;
    output_length: Expression | number;
  }

  export type ProfileVersionRequired =
    | FunctionProfileVersionRequired
    | VectorCompletionProfile;

  export type ProfileVersionOptional =
    | FunctionProfileVersionOptional
    | VectorCompletionProfile;

  export type FunctionProfileVersionRequired =
    | {
        function_author: string;
        function_id: string;
        author: string;
        id: string;
        version: number;
      }
    | ProfileVersionRequired[];

  export type FunctionProfileVersionOptional =
    | {
        function_author: string;
        function_id: string;
        author: string;
        id: string;
        version?: number | null;
      }
    | ProfileVersionOptional[];

  export interface VectorCompletionProfile {
    ensemble: Vector.Completions.Request.Ensemble;
    profile: number[];
  }

  export interface Expression {
    $jmespath: string;
  }

  export type InputSchema =
    | InputSchema.Object
    | InputSchema.Array
    | InputSchema.String
    | InputSchema.Number
    | InputSchema.Integer
    | InputSchema.Boolean
    | InputSchema.Image
    | InputSchema.Audio
    | InputSchema.Video
    | InputSchema.File;

  export namespace InputSchema {
    export interface Object {
      type: "object";
      description?: string | null;
      properties: Record<string, InputSchema>;
      required?: string[] | null;
    }

    export interface Array {
      type: "array";
      description?: string | null;
      minItems?: number | null;
      maxItems?: number | null;
      items: InputSchema;
    }

    export interface String {
      type: "string";
      description?: string | null;
      enum?: string[] | null;
    }

    export interface Number {
      type: "number";
      description?: string | null;
      minimum?: number | null;
      maximum?: number | null;
    }

    export interface Integer {
      type: "integer";
      description?: string | null;
      minimum?: number | null;
      maximum?: number | null;
    }

    export interface Boolean {
      type: "boolean";
      description?: string | null;
    }

    export interface Image {
      type: "image";
      description?: string | null;
    }

    export interface Audio {
      type: "audio";
      description?: string | null;
    }

    export interface Video {
      type: "video";
      description?: string | null;
    }

    export interface File {
      type: "file";
      description?: string | null;
    }
  }

  export type InputMaps = Expression | Expression[];

  export type TaskExpression =
    | TaskExpression.ScalarFunction
    | TaskExpression.VectorFunction
    | TaskExpression.VectorCompletion;

  export namespace TaskExpression {
    export interface ScalarFunction {
      type: "scalar.function";
      author: string;
      id: string;
      version: number;
      skip?: Expression | null;
      map?: number | null;
      input: Expression | InputExpression;
    }

    export interface VectorFunction {
      type: "vector.function";
      author: string;
      id: string;
      version: number;
      skip?: Expression | null;
      map?: number | null;
      input: Expression | InputExpression;
    }

    export interface VectorCompletion {
      type: "vector.completion";
      skip?: Expression | null;
      map?: number | null;
      messages: Expression | (Expression | MessageExpression)[];
      tools?: Expression | (Expression | ToolExpression)[] | null;
      responses:
        | Expression
        | (Expression | MessageExpression.RichContentExpression)[];
    }

    export type InputExpression =
      | Message.RichContentPart
      | { [key: string]: Expression | InputExpression }
      | (Expression | InputExpression)[]
      | string
      | number
      | boolean;

    export type MessageExpression =
      | MessageExpression.DeveloperExpression
      | MessageExpression.SystemExpression
      | MessageExpression.UserExpression
      | MessageExpression.ToolExpression
      | MessageExpression.AssistantExpression;

    export namespace MessageExpression {
      export type SimpleContentExpression =
        | string
        | (Expression | Message.SimpleContentPart)[];

      export type RichContentExpression =
        | string
        | (Expression | Message.RichContentPart)[];

      export interface DeveloperExpression {
        role: "developer";
        content: Expression | SimpleContentExpression;
        name?: Expression | string | null;
      }

      export interface SystemExpression {
        role: "system";
        content: Expression | SimpleContentExpression;
        name?: Expression | string | null;
      }

      export interface UserExpression {
        role: "user";
        content: Expression | RichContentExpression;
        name?: Expression | string | null;
      }

      export interface ToolExpression {
        role: "tool";
        content: Expression | RichContentExpression;
        tool_call_id: Expression | string;
      }

      export interface AssistantExpression {
        role: "assistant";
        content?: Expression | RichContentExpression;
        name?: Expression | string | null;
        refusal?: Expression | string | null;
        tool_calls?:
          | Expression
          | (Expression | AssistantExpression.ToolCallExpression)[]
          | null;
        reasoning?: Expression | string | null;
      }

      export namespace AssistantExpression {
        export type ToolCallExpression = ToolCallExpression.FunctionExpression;

        export namespace ToolCallExpression {
          export interface FunctionExpression {
            type: "function";
            id: Expression | string;
            function: Expression | FunctionExpression.DefinitionExpression;
          }

          export namespace FunctionExpression {
            export interface DefinitionExpression {
              name: Expression | string;
              arguments: Expression | string;
            }
          }
        }
      }
    }

    export type ToolExpression = ToolExpression.FunctionExpression;

    export namespace ToolExpression {
      export interface FunctionExpression {
        type: "function";
        function: Expression | FunctionExpression.DefinitionExpression;
      }

      export namespace FunctionExpression {
        export interface DefinitionExpression {
          name: Expression | string;
          description?: Expression | string | null;
          parameters?:
            | Expression
            | { [key: string]: JsonValueExpression }
            | null;
          strict?: Expression | boolean | null;
        }

        export type JsonValueExpression =
          | null
          | boolean
          | number
          | string
          | (Expression | JsonValueExpression)[]
          | { [key: string]: Expression | JsonValueExpression };
      }
    }
  }

  export namespace Executions {
    export namespace Request {
      export interface FunctionExecutionParamsBase {
        retry_token?: string | null;
        input: Input;
        provider?: Chat.Completions.Request.Provider | null;
        seed?: number | null;
        backoff_max_elapsed_time?: number | null;
        first_chunk_timeout?: number | null;
        other_chunk_timeout?: number | null;
      }

      export type Input =
        | Message.RichContentPart
        | { [key: string]: Input }
        | Input[]
        | string
        | number
        | boolean;

      // Execute Inline Function

      export interface FunctionExecutionParamsExecuteInlineBase
        extends FunctionExecutionParamsBase {
        function: Function;
        profile: FunctionProfileVersionOptional;
      }

      export interface FunctionExecutionParamsExecuteInlineStreaming
        extends FunctionExecutionParamsExecuteInlineBase {
        stream: true;
      }

      export interface FunctionExecutionParamsExecuteInlineNonStreaming
        extends FunctionExecutionParamsExecuteInlineBase {
        stream?: false | null;
      }

      export type FunctionExecutionParamsExecuteInline =
        | FunctionExecutionParamsExecuteInlineStreaming
        | FunctionExecutionParamsExecuteInlineNonStreaming;

      // Execute Published Function

      export interface FunctionExecutionParamsExecuteBase
        extends FunctionExecutionParamsBase {
        profile?: FunctionProfileVersionOptional | null;
      }

      export interface FunctionExecutionParamsExecuteStreaming
        extends FunctionExecutionParamsExecuteBase {
        stream: true;
      }

      export interface FunctionExecutionParamsExecuteNonStreaming
        extends FunctionExecutionParamsExecuteBase {
        stream?: false | null;
      }

      export type FunctionExecutionParamsExecute =
        | FunctionExecutionParamsExecuteStreaming
        | FunctionExecutionParamsExecuteNonStreaming;

      // Publish Scalar Function

      export interface FunctionExecutionParamsPublishScalarFunctionBase
        extends FunctionExecutionParamsBase {
        function: Scalar;
        publish_function: {
          description: string;
          changelog?: string | null;
          input_schema: InputSchema;
        };
        profile: ProfileVersionRequired[];
        publish_profile: {
          id: "default";
          version: number;
          description: string;
          changelog?: string | null;
        };
      }

      export interface FunctionExecutionParamsPublishScalarFunctionStreaming
        extends FunctionExecutionParamsPublishScalarFunctionBase {
        stream: true;
      }

      export interface FunctionExecutionParamsPublishScalarFunctionNonStreaming
        extends FunctionExecutionParamsPublishScalarFunctionBase {
        stream?: false | null;
      }

      export type FunctionExecutionParamsPublishScalarFunction =
        | FunctionExecutionParamsPublishScalarFunctionStreaming
        | FunctionExecutionParamsPublishScalarFunctionNonStreaming;

      // Publish Vector Function

      export interface FunctionExecutionParamsPublishVectorFunctionBase
        extends FunctionExecutionParamsBase {
        function: Vector;
        publish_function: {
          description: string;
          changelog?: string | null;
          input_schema: InputSchema;
          output_length: Expression | number;
        };
        profile: ProfileVersionRequired[];
        publish_profile: {
          id: "default";
          version: number;
          description: string;
          changelog?: string | null;
        };
      }

      export interface FunctionExecutionParamsPublishVectorFunctionStreaming
        extends FunctionExecutionParamsPublishVectorFunctionBase {
        stream: true;
      }

      export interface FunctionExecutionParamsPublishVectorFunctionNonStreaming
        extends FunctionExecutionParamsPublishVectorFunctionBase {
        stream?: false | null;
      }

      export type FunctionExecutionParamsPublishVectorFunction =
        | FunctionExecutionParamsPublishVectorFunctionStreaming
        | FunctionExecutionParamsPublishVectorFunctionNonStreaming;

      // Publish Function

      export type FunctionExecutionParamsPublishFunctionStreaming =
        | FunctionExecutionParamsPublishScalarFunctionStreaming
        | FunctionExecutionParamsPublishVectorFunctionStreaming;

      export type FunctionExecutionParamsPublishFunctionNonStreaming =
        | FunctionExecutionParamsPublishScalarFunctionNonStreaming
        | FunctionExecutionParamsPublishVectorFunctionNonStreaming;

      export type FunctionExecutionParamsPublishFunction =
        | FunctionExecutionParamsPublishScalarFunction
        | FunctionExecutionParamsPublishVectorFunction;

      // Publish Profile

      export interface FunctionExecutionParamsPublishProfileBase
        extends FunctionExecutionParamsBase {
        profile: ProfileVersionRequired[];
        publish_profile: {
          id: string;
          version: number;
          description: string;
          changelog?: string | null;
        };
      }

      export interface FunctionExecutionParamsPublishProfileStreaming
        extends FunctionExecutionParamsPublishProfileBase {
        stream: true;
      }

      export interface FunctionExecutionParamsPublishProfileNonStreaming
        extends FunctionExecutionParamsPublishProfileBase {
        stream?: false | null;
      }

      export type FunctionExecutionParamsPublishProfile =
        | FunctionExecutionParamsPublishProfileStreaming
        | FunctionExecutionParamsPublishProfileNonStreaming;
    }

    export namespace Response {
      export namespace Streaming {
        export interface FunctionExecutionChunk {
          id: string;
          tasks: TaskChunk[];
          tasks_errors?: boolean;
          output?: number | number[] | JsonValue;
          error?: ObjectiveAIError;
          retry_token?: string;
          function_published?: boolean;
          profile_published?: boolean;
          created: number;
          function: string | null;
          profile: string | null;
          object:
            | "scalar.function.execution.chunk"
            | "vector.function.execution.chunk";
          usage?: Vector.Completions.Response.Usage;
        }

        export namespace FunctionExecutionChunk {
          export function merged(
            a: FunctionExecutionChunk,
            b: FunctionExecutionChunk
          ): [FunctionExecutionChunk, boolean] {
            const id = a.id;
            const [tasks, tasksChanged] = TaskChunk.mergedList(
              a.tasks,
              b.tasks
            );
            const [tasks_errors, tasks_errorsChanged] = merge(
              a.tasks_errors,
              b.tasks_errors
            );
            const [output, outputChanged] = merge(a.output, b.output);
            const [error, errorChanged] = merge(a.error, b.error);
            const [retry_token, retry_tokenChanged] = merge(
              a.retry_token,
              b.retry_token
            );
            const [function_published, function_publishedChanged] = merge(
              a.function_published,
              b.function_published
            );
            const [profile_published, profile_publishedChanged] = merge(
              a.profile_published,
              b.profile_published
            );
            const created = a.created;
            const function_ = a.function;
            const profile = a.profile;
            const object = a.object;
            const [usage, usageChanged] = merge(a.usage, b.usage);
            if (
              tasksChanged ||
              tasks_errorsChanged ||
              outputChanged ||
              errorChanged ||
              retry_tokenChanged ||
              function_publishedChanged ||
              profile_publishedChanged ||
              usageChanged
            ) {
              return [
                {
                  id,
                  tasks,
                  ...(tasks_errors !== undefined ? { tasks_errors } : {}),
                  ...(output !== undefined ? { output } : {}),
                  ...(error !== undefined ? { error } : {}),
                  ...(retry_token !== undefined ? { retry_token } : {}),
                  ...(function_published !== undefined
                    ? { function_published }
                    : {}),
                  ...(profile_published !== undefined
                    ? { profile_published }
                    : {}),
                  created,
                  function: function_,
                  profile,
                  object,
                  ...(usage !== undefined ? { usage } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }
        }

        export type TaskChunk = TaskChunk.Function | TaskChunk.VectorCompletion;

        export namespace TaskChunk {
          export function merged(
            a: TaskChunk,
            b: TaskChunk
          ): [TaskChunk, boolean] {
            if ("scores" in a) {
              return VectorCompletion.merged(a, b as VectorCompletion);
            } else {
              return Function.merged(a, b as Function);
            }
          }

          export function mergedList(
            a: TaskChunk[],
            b: TaskChunk[]
          ): [TaskChunk[], boolean] {
            let merged: TaskChunk[] | undefined = undefined;
            for (const chunk of b) {
              const existingIndex = a.findIndex(
                ({ index }) => index === chunk.index
              );
              if (existingIndex === -1) {
                if (merged === undefined) {
                  merged = [...a, chunk];
                } else {
                  merged.push(chunk);
                }
              } else {
                const [mergedChunk, chunkChanged] = TaskChunk.merged(
                  a[existingIndex],
                  chunk
                );
                if (chunkChanged) {
                  if (merged === undefined) {
                    merged = [...a];
                  }
                  merged[existingIndex] = mergedChunk;
                }
              }
            }
            return merged ? [merged, true] : [a, false];
          }

          export interface Function extends FunctionExecutionChunk {
            index: number;
            task_index: number;
            task_path: number[];
          }

          export namespace Function {
            export function merged(
              a: Function,
              b: Function
            ): [Function, boolean] {
              const index = a.index;
              const task_index = a.task_index;
              const task_path = a.task_path;
              const [base, baseChanged] = FunctionExecutionChunk.merged(a, b);
              if (baseChanged) {
                return [
                  {
                    index,
                    task_index,
                    task_path,
                    ...base,
                  },
                  true,
                ];
              } else {
                return [a, false];
              }
            }
          }

          export interface VectorCompletion
            extends Vector.Completions.Response.Streaming
              .VectorCompletionChunk {
            index: number;
            task_index: number;
            task_path: number[];
            error?: ObjectiveAIError;
          }

          export namespace VectorCompletion {
            export function merged(
              a: VectorCompletion,
              b: VectorCompletion
            ): [VectorCompletion, boolean] {
              const index = a.index;
              const task_index = a.task_index;
              const task_path = a.task_path;
              const [base, baseChanged] =
                Vector.Completions.Response.Streaming.VectorCompletionChunk.merged(
                  a,
                  b
                );
              const [error, errorChanged] = merge(a.error, b.error);
              if (baseChanged || errorChanged) {
                return [
                  {
                    index,
                    task_index,
                    task_path,
                    ...base,
                    ...(error !== undefined ? { error } : {}),
                  },
                  true,
                ];
              } else {
                return [a, false];
              }
            }
          }
        }
      }

      export namespace Unary {
        export interface FunctionExecution {
          id: string;
          tasks: Task[];
          tasks_errors: boolean;
          output: number | number[] | JsonValue;
          error: ObjectiveAIError | null;
          retry_token: string | null;
          function_published?: boolean;
          profile_published?: boolean;
          created: number;
          function: string | null;
          profile: string | null;
          object: "scalar.function.execution" | "vector.function.execution";
          usage: Vector.Completions.Response.Usage;
        }

        export type Task = Task.Function | Task.VectorCompletion;

        export namespace Task {
          export interface Function extends FunctionExecution {
            index: number;
            task_index: number;
            task_path: number[];
          }

          export interface VectorCompletion
            extends Vector.Completions.Response.Unary.VectorCompletion {
            index: number;
            task_index: number;
            task_path: number[];
            error: ObjectiveAIError | null;
          }
        }
      }
    }
  }

  export namespace ComputeProfile {
    export namespace Request {
      export interface FunctionComputeProfileParamsBase {
        retry_token?: string | null;
        max_retries?: number | null;
        n: number;
        dataset: DatasetItem[];
        ensemble: Vector.Completions.Request.Ensemble;
        provider?: Chat.Completions.Request.Provider | null;
        seed?: number | null;
        backoff_max_elapsed_time?: number | null;
        first_chunk_timeout?: number | null;
        other_chunk_timeout?: number | null;
      }

      export interface FunctionComputeProfileParamsStreaming
        extends FunctionComputeProfileParamsBase {
        stream: true;
      }

      export interface FunctionComputeProfileParamsNonStreaming
        extends FunctionComputeProfileParamsBase {
        stream?: false | null;
      }

      export type FunctionComputeProfileParams =
        | FunctionComputeProfileParamsStreaming
        | FunctionComputeProfileParamsNonStreaming;

      export interface DatasetItem {
        input: Executions.Request.Input;
        target: DatasetItem.Target;
      }

      export namespace DatasetItem {
        export type Target =
          | Target.Scalar
          | Target.Vector
          | Target.VectorWinner;

        export namespace Target {
          export interface Scalar {
            type: "scalar";
            value: number;
          }

          export interface Vector {
            type: "vector";
            value: number[];
          }

          export interface VectorWinner {
            type: "vector_winner";
            value: number;
          }
        }
      }
    }

    export namespace Response {
      export namespace Streaming {
        export interface FunctionComputeProfileChunk {
          id: string;
          executions: FunctionExecutionChunk[];
          executions_errors?: boolean;
          profile?: ProfileVersionRequired[];
          fitting_stats?: FittingStats;
          created: number;
          function: string;
          object: "function.compute.profile.chunk";
          usage?: Vector.Completions.Response.Usage;
        }

        export namespace FunctionComputeProfileChunk {
          export function merged(
            a: FunctionComputeProfileChunk,
            b: FunctionComputeProfileChunk
          ): [FunctionComputeProfileChunk, boolean] {
            const id = a.id;
            const [executions, executionsChanged] =
              FunctionExecutionChunk.mergedList(a.executions, b.executions);
            const [executions_errors, executions_errorsChanged] = merge(
              a.executions_errors,
              b.executions_errors
            );
            const [profile, profileChanged] = merge(a.profile, b.profile);
            const [fitting_stats, fitting_statsChanged] = merge(
              a.fitting_stats,
              b.fitting_stats
            );
            const created = a.created;
            const function_ = a.function;
            const object = a.object;
            const [usage, usageChanged] = merge(a.usage, b.usage);
            if (
              executionsChanged ||
              executions_errorsChanged ||
              profileChanged ||
              fitting_statsChanged ||
              usageChanged
            ) {
              return [
                {
                  id,
                  executions,
                  ...(executions_errors !== undefined
                    ? { executions_errors }
                    : {}),
                  ...(profile !== undefined ? { profile } : {}),
                  ...(fitting_stats !== undefined ? { fitting_stats } : {}),
                  created,
                  function: function_,
                  object,
                  ...(usage !== undefined ? { usage } : {}),
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }
        }

        export interface FunctionExecutionChunk
          extends Executions.Response.Streaming.FunctionExecutionChunk {
          index: number;
          dataset: number;
          n: number;
          retry: number;
        }

        export namespace FunctionExecutionChunk {
          export function merged(
            a: FunctionExecutionChunk,
            b: FunctionExecutionChunk
          ): [FunctionExecutionChunk, boolean] {
            const index = a.index;
            const dataset = a.dataset;
            const n = a.n;
            const retry = a.retry;
            const [base, baseChanged] =
              Executions.Response.Streaming.FunctionExecutionChunk.merged(a, b);
            if (baseChanged) {
              return [
                {
                  index,
                  dataset,
                  n,
                  retry,
                  ...base,
                },
                true,
              ];
            } else {
              return [a, false];
            }
          }

          export function mergedList(
            a: FunctionExecutionChunk[],
            b: FunctionExecutionChunk[]
          ): [FunctionExecutionChunk[], boolean] {
            let merged: FunctionExecutionChunk[] | undefined = undefined;
            for (const chunk of b) {
              const existingIndex = a.findIndex(
                ({ index }) => index === chunk.index
              );
              if (existingIndex === -1) {
                if (merged === undefined) {
                  merged = [...a, chunk];
                } else {
                  merged.push(chunk);
                }
              } else {
                const [mergedChunk, chunkChanged] =
                  FunctionExecutionChunk.merged(a[existingIndex], chunk);
                if (chunkChanged) {
                  if (merged === undefined) {
                    merged = [...a];
                  }
                  merged[existingIndex] = mergedChunk;
                }
              }
            }
            return merged ? [merged, true] : [a, false];
          }
        }
      }

      export namespace Unary {
        export interface FunctionComputeProfile {
          id: string;
          executions: FunctionExecution[];
          executions_errors: boolean;
          profile: ProfileVersionRequired[];
          fitting_stats: FittingStats;
          created: number;
          function: string;
          object: "function.compute.profile";
          usage: Vector.Completions.Response.Usage;
        }

        export interface FunctionExecution
          extends Executions.Response.Unary.FunctionExecution {
          index: number;
          dataset: number;
          n: number;
          retry: number;
        }
      }

      export interface FittingStats {
        loss: number;
        executions: number;
        starts: number;
        rounds: number;
        errors: number;
      }
    }
  }

  export namespace Profile {
    export interface ListItem {
      function_author: string;
      function_id: string;
      author: string;
      id: string;
      version: number;
    }

    export async function list(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<ListItem[]> {
      const response = await openai.get("/functions/profiles", options);
      return response as ListItem[];
    }

    export interface RetrieveItem {
      created: number;
      shape: string;
      function_author: string;
      function_id: string;
      author: string;
      id: string;
      version: number;
      profile: Function.ProfileVersionRequired[];
    }

    export async function retrieve(
      openai: OpenAI,
      function_author: string,
      function_id: string,
      author: string,
      id: string,
      version?: number | null | undefined,
      options?: OpenAI.RequestOptions
    ): Promise<RetrieveItem> {
      const response = await openai.get(
        version !== null && version !== undefined
          ? `/functions/${function_author}/${function_id}/profiles/${author}/${id}/${version}`
          : `/functions/${function_author}/${function_id}/profiles/${author}/${id}`,
        options
      );
      return response as RetrieveItem;
    }

    export interface HistoricalUsage {
      requests: number;
      completion_tokens: number;
      prompt_tokens: number;
      total_cost: number;
    }

    export async function retrieveUsage(
      openai: OpenAI,
      function_author: string,
      function_id: string,
      author: string,
      id: string,
      version?: number | null | undefined,
      options?: OpenAI.RequestOptions
    ): Promise<HistoricalUsage> {
      const response = await openai.get(
        version !== null && version !== undefined
          ? `/functions/${function_author}/${function_id}/profiles/${author}/${id}/${version}/usage`
          : `/functions/${function_author}/${function_id}/profiles/${author}/${id}/usage`,
        options
      );
      return response as HistoricalUsage;
    }
  }

  export async function executeInline(
    openai: OpenAI,
    body: Executions.Request.FunctionExecutionParamsExecuteInlineStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function executeInline(
    openai: OpenAI,
    body: Executions.Request.FunctionExecutionParamsExecuteInlineNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function executeInline(
    openai: OpenAI,
    body: Executions.Request.FunctionExecutionParamsExecuteInline,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    const response = await openai.post("/functions", {
      body,
      stream: body.stream ?? false,
      ...options,
    });
    return response as
      | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
      | Executions.Response.Unary.FunctionExecution;
  }

  export async function execute(
    openai: OpenAI,
    author: string,
    id: string,
    version: number | null | undefined,
    body: Executions.Request.FunctionExecutionParamsExecuteStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function execute(
    openai: OpenAI,
    author: string,
    id: string,
    version: number | null | undefined,
    body: Executions.Request.FunctionExecutionParamsExecuteNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function execute(
    openai: OpenAI,
    author: string,
    id: string,
    version: number | null | undefined,
    body: Executions.Request.FunctionExecutionParamsExecute,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    const response = await openai.post(
      version !== null && version !== undefined
        ? `/functions/${author}/${id}/${version}`
        : `/functions/${author}/${id}`,
      {
        body,
        stream: body.stream ?? false,
        ...options,
      }
    );
    return response as
      | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
      | Executions.Response.Unary.FunctionExecution;
  }

  export async function publishFunction(
    openai: OpenAI,
    author: string,
    id: string,
    version: number,
    body: Executions.Request.FunctionExecutionParamsPublishFunctionStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function publishFunction(
    openai: OpenAI,
    author: string,
    id: string,
    version: number,
    body: Executions.Request.FunctionExecutionParamsPublishFunctionNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function publishFunction(
    openai: OpenAI,
    author: string,
    id: string,
    version: number,
    body: Executions.Request.FunctionExecutionParamsPublishFunction,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    const response = await openai.post(
      `/functions/${author}/${id}/${version}/publish`,
      {
        body,
        stream: body.stream ?? false,
        ...options,
      }
    );
    return response as
      | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
      | Executions.Response.Unary.FunctionExecution;
  }

  export async function publishProfile(
    openai: OpenAI,
    function_author: string,
    function_id: string,
    body: Executions.Request.FunctionExecutionParamsPublishProfileStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function publishProfile(
    openai: OpenAI,
    function_author: string,
    function_id: string,
    body: Executions.Request.FunctionExecutionParamsPublishProfileNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function publishProfile(
    openai: OpenAI,
    function_author: string,
    function_id: string,
    body: Executions.Request.FunctionExecutionParamsPublishProfile,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    const response = await openai.post(
      `/functions/${function_author}/${function_id}/profiles/publish`,
      {
        body,
        stream: body.stream ?? false,
        ...options,
      }
    );
    return response as
      | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
      | Executions.Response.Unary.FunctionExecution;
  }

  export async function computeProfile(
    openai: OpenAI,
    author: string,
    id: string,
    version: number | null | undefined,
    body: ComputeProfile.Request.FunctionComputeProfileParamsStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<
    Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
  >;
  export async function computeProfile(
    openai: OpenAI,
    author: string,
    id: string,
    version: number | null | undefined,
    body: ComputeProfile.Request.FunctionComputeProfileParamsNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<ComputeProfile.Response.Unary.FunctionComputeProfile>;
  export async function computeProfile(
    openai: OpenAI,
    author: string,
    id: string,
    version: number | null | undefined,
    body: ComputeProfile.Request.FunctionComputeProfileParams,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
    | ComputeProfile.Response.Unary.FunctionComputeProfile
  > {
    const response = await openai.post(
      version !== null && version !== undefined
        ? `/functions/${author}/${id}/${version}/profiles/compute`
        : `/functions/${author}/${id}/profiles/compute`,
      {
        body,
        stream: body.stream ?? false,
        ...options,
      }
    );
    return response as
      | Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
      | ComputeProfile.Response.Unary.FunctionComputeProfile;
  }

  export interface ListItem {
    author: string;
    id: string;
    version: number;
  }

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/functions", options);
    return response as { data: ListItem[] };
  }

  export interface ScalarRetrieveItem extends Scalar {
    created: number;
    shape: string;
  }

  export interface VectorRetrieveItem extends Vector {
    created: number;
    shape: string;
  }

  export type RetrieveItem = ScalarRetrieveItem | VectorRetrieveItem;

  export async function retrieve(
    openai: OpenAI,
    author: string,
    id: string,
    version?: number | null | undefined,
    options?: OpenAI.RequestOptions
  ): Promise<RetrieveItem> {
    const response = await openai.get(
      version !== null && version !== undefined
        ? `/functions/${author}/${id}/${version}`
        : `/functions/${author}/${id}`,
      options
    );
    return response as RetrieveItem;
  }

  export interface HistoricalUsage {
    requests: number;
    completion_tokens: number;
    prompt_tokens: number;
    total_cost: number;
  }

  export async function retrieveUsage(
    openai: OpenAI,
    author: string,
    id: string,
    version?: number | null | undefined,
    options?: OpenAI.RequestOptions
  ): Promise<HistoricalUsage> {
    const response = await openai.get(
      version !== null && version !== undefined
        ? `/functions/${author}/${id}/${version}/usage`
        : `/functions/${author}/${id}/usage`,
      options
    );
    return response as HistoricalUsage;
  }
}

export namespace Auth {
  export interface ApiKey {
    api_key: string;
    created: string; // RFC 3339 timestamp
    expires: string | null; // RFC 3339 timestamp
    disabled: string | null; // RFC 3339 timestamp
    name: string;
    description: string | null;
  }

  export interface ApiKeyWithCost extends ApiKey {
    cost: number;
  }

  export namespace ApiKey {
    export async function list(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<{ data: ApiKeyWithCost[] }> {
      const response = await openai.get("/auth/keys", options);
      return response as { data: ApiKeyWithCost[] };
    }

    export async function create(
      openai: OpenAI,
      name: string,
      expires?: Date | null,
      description?: string | null,
      options?: OpenAI.RequestOptions
    ): Promise<ApiKey> {
      const response = await openai.post("/auth/keys", {
        body: {
          name,
          expires,
          description,
        },
        ...options,
      });
      return response as ApiKey;
    }

    export async function remove(
      openai: OpenAI,
      key: string,
      options?: OpenAI.RequestOptions
    ): Promise<ApiKey> {
      const response = await openai.delete("/auth/keys", {
        body: {
          api_key: key,
        },
        ...options,
      });
      return response as ApiKey;
    }
  }

  export interface OpenRouterApiKey {
    api_key: string;
  }

  export namespace OpenRouterApiKey {
    export async function retrieve(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<OpenRouterApiKey> {
      const response = await openai.get("/auth/keys/openrouter", options);
      return response as OpenRouterApiKey;
    }

    export async function create(
      openai: OpenAI,
      apiKey: string,
      options?: OpenAI.RequestOptions
    ): Promise<OpenRouterApiKey> {
      const response = await openai.post("/auth/keys/openrouter", {
        body: {
          api_key: apiKey,
        },
        ...options,
      });
      return response as OpenRouterApiKey;
    }

    export async function remove(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<OpenRouterApiKey> {
      const response = await openai.delete("/auth/keys/openrouter", options);
      return response as OpenRouterApiKey;
    }
  }

  export interface Credits {
    credits: number;
    total_credits_purchased: number;
    total_credits_used: number;
  }

  export namespace Credits {
    export async function retrieve(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<Credits> {
      const response = await openai.get("/auth/credits", options);
      return response as Credits;
    }
  }

  export namespace Username {
    export async function retrieve(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<{ username: string | null }> {
      const response = await openai.get("/auth/username", options);
      return response as { username: string | null };
    }

    export async function set(
      openai: OpenAI,
      username: string,
      options?: OpenAI.RequestOptions
    ): Promise<{ username: string }> {
      const response = await openai.post("/auth/username", {
        body: { username },
        ...options,
      });
      return response as { username: string };
    }
  }
}

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface ObjectiveAIError {
  code: number;
  message: JsonValue;
}

function merge<T extends {}>(
  a: T,
  b: T,
  combine?: (a: T, b: T) => [T, boolean]
): [T, boolean];
function merge<T extends {}>(
  a: T | null,
  b: T | null,
  combine?: (a: T, b: T) => [T, boolean]
): [T | null, boolean];
function merge<T extends {}>(
  a: T | undefined,
  b: T | undefined,
  combine?: (a: T, b: T) => [T, boolean]
): [T | undefined, boolean];
function merge<T extends {}>(
  a: T | null | undefined,
  b: T | null | undefined,
  combine?: (a: T, b: T) => [T, boolean]
): [T | null | undefined, boolean];
function merge<T extends {}>(
  a: T | null | undefined,
  b: T | null | undefined,
  combine?: (a: T, b: T) => [T, boolean]
): [T | null | undefined, boolean] {
  if (a !== null && a !== undefined && b !== null && b !== undefined) {
    return combine ? combine(a, b) : [a, false];
  } else if (a !== null && a !== undefined) {
    return [a, false];
  } else if (b !== null && b !== undefined) {
    return [b, true];
  } else if (a === null || b === null) {
    return [null, false];
  } else {
    return [undefined, false];
  }
}

function mergedString(a: string, b: string): [string, boolean] {
  return b === "" ? [a, false] : [a + b, true];
}
// function mergedNumber(a: number, b: number): [number, boolean] {
//   return b === 0 ? [a, false] : [a + b, true];
// }
