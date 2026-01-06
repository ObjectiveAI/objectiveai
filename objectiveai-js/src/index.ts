import OpenAI from "openai";
import { Stream } from "openai/streaming";
import z from "zod";

// Message

export const MessageSchema = z
  .union([
    Message.DeveloperSchema,
    Message.SystemSchema,
    Message.UserSchema,
    Message.ToolSchema,
    Message.AssistantSchema,
  ])
  .describe("A message exchanged in a chat conversation.");
export type Message = z.infer<typeof MessageSchema>;
// export type Message =
//   | Message.Developer
//   | Message.System
//   | Message.User
//   | Message.Tool
//   | Message.Assistant;

export namespace Message {
  export const SimpleContentPartSchema = z.object({
    type: z.literal("text"),
    text: z.string().describe("The text content."),
  });
  export type SimpleContentPart = z.infer<typeof SimpleContentPartSchema>;
  // export interface SimpleContentPart {
  //   type: "text";
  //   text: string;
  // }

  export const SimpleContentSchema = z
    .union([
      z.string().describe("String content."),
      z.array(SimpleContentPartSchema).describe("An array of content parts."),
    ])
    .describe("Simple content which is plain text.");
  export type SimpleContent = z.infer<typeof SimpleContentSchema>;
  // export type SimpleContent = string | SimpleContentPart[];

  export const RichContentPartSchema = z.union([
    RichContentPart.TextSchema,
    RichContentPart.ImageUrlSchema,
    RichContentPart.InputAudioSchema,
    RichContentPart.InputVideoSchema,
    RichContentPart.FileSchema,
  ]);
  export type RichContentPart = z.infer<typeof RichContentPartSchema>;
  // export type RichContentPart =
  //   | RichContentPart.Text
  //   | RichContentPart.ImageUrl
  //   | RichContentPart.InputAudio
  //   | RichContentPart.InputVideo
  //   | RichContentPart.File;

  export const RichContentSchema = z
    .union([
      z.string().describe("String content."),
      z.array(RichContentPartSchema).describe("An array of content parts."),
    ])
    .describe("Rich content which can include various media types.");
  export type RichContent = z.infer<typeof RichContentSchema>;
  // export type RichContent = string | RichContentPart[];

  export namespace RichContentPart {
    export const TextSchema = z.object({
      type: z.literal("text"),
      text: z.string().describe("The text content."),
    });
    export type Text = z.infer<typeof TextSchema>;
    // export interface Text {
    //   type: "text";
    //   text: string;
    // }

    export const ImageUrlSchema = z.object({
      type: z.literal("image_url"),
      image_url: ImageUrl.DefinitionSchema,
    });
    export type ImageUrl = z.infer<typeof ImageUrlSchema>;
    // export interface ImageUrl {
    //   type: "image_url";
    //   image_url: ImageUrl.Definition;
    // }

    export namespace ImageUrl {
      export const DetailSchema = z
        .enum(["auto", "low", "high"])
        .describe("Specifies the detail level of the image.");
      export type Detail = z.infer<typeof DetailSchema>;

      export const DefinitionSchema = z.object({
        url: z
          .string()
          .describe(
            "Either a URL of the image or the base64 encoded image data."
          ),
        detail: DetailSchema.optional().nullable(),
      });
      export type Definition = z.infer<typeof DefinitionSchema>;
    }

    export const InputAudioSchema = z.object({
      type: z.literal("input_audio"),
      input_audio: InputAudio.DefinitionSchema,
    });
    export type InputAudio = z.infer<typeof InputAudioSchema>;
    // export interface InputAudio {
    //   type: "input_audio";
    //   input_audio: InputAudio.Definition;
    // }

    export namespace InputAudio {
      export const FormatSchema = z
        .enum(["wav", "mp3"])
        .describe("The format of the encoded audio data.");
      export type Format = z.infer<typeof FormatSchema>;
      // export type Format = "wav" | "mp3";

      export const DefinitionSchema = z.object({
        data: z.string().describe("Base64 encoded audio data."),
        format: FormatSchema,
      });
      export type Definition = z.infer<typeof DefinitionSchema>;
      // export interface Definition {
      //   data: string;
      //   format: Format;
      // }
    }

    export const InputVideoSchema = z.object({
      type: z.enum(["input_video", "video_url"]),
      input_video: InputVideo.DefinitionSchema,
    });
    export type InputVideo = z.infer<typeof InputVideoSchema>;
    // export interface InputVideo {
    //   type: "input_video";
    //   video_url: InputVideo.Definition;
    // }

    export namespace InputVideo {
      export const DefinitionSchema = z.object({
        url: z.string().describe("URL of the video."),
      });
      export type Definition = z.infer<typeof DefinitionSchema>;
      // export interface Definition {
      //   url: string;
      // }
    }

    // export interface VideoUrl {
    //   type: "video_url";
    //   video_url: InputVideo.Definition;
    // }

    export const FileSchema = z.object({
      type: z.literal("file"),
      file: File.DefinitionSchema,
    });
    export type File = z.infer<typeof FileSchema>;
    // export interface File {
    //   type: "file";
    //   file: File.Definition;
    // }

    export namespace File {
      export const DefinitionSchema = z.object({
        file_data: z
          .string()
          .optional()
          .nullable()
          .describe(
            "The base64 encoded file data, used when passing the file to the model as a string."
          ),
        file_id: z
          .string()
          .optional()
          .nullable()
          .describe("The ID of an uploaded file to use as input."),
        filename: z
          .string()
          .optional()
          .nullable()
          .describe(
            "The name of the file, used when passing the file to the model as a string."
          ),
        file_url: z
          .string()
          .optional()
          .nullable()
          .describe(
            "The URL of the file, used when passing the file to the model as a URL."
          ),
      });
      export type Definition = z.infer<typeof DefinitionSchema>;
    }
  }

  export const NameSchema = z
    .string()
    .describe(
      "An optional name for the participant. Provides the model information to differentiate between participants of the same role."
    );
  export type Name = z.infer<typeof NameSchema>;

  export const DeveloperSchema = z
    .object({
      role: z.literal("developer"),
      content: SimpleContentSchema,
      name: NameSchema.optional().nullable(),
    })
    .describe(
      "Developer-provided instructions that the model should follow, regardless of messages sent by the user."
    );
  export type Developer = z.infer<typeof DeveloperSchema>;
  // export interface Developer {
  //   role: "developer";
  //   content: SimpleContent;
  //   name?: string;
  // }

  export const SystemSchema = z
    .object({
      role: z.literal("system"),
      content: SimpleContentSchema,
      name: NameSchema.optional().nullable(),
    })
    .describe(
      "Developer-provided instructions that the model should follow, regardless of messages sent by the user."
    );
  export type System = z.infer<typeof SystemSchema>;
  // export interface System {
  //   role: "system";
  //   content: SimpleContent;
  //   name?: string;
  // }

  export const UserSchema = z
    .object({
      role: z.literal("user"),
      content: RichContentSchema,
      name: NameSchema.optional().nullable(),
    })
    .describe(
      "Messages sent by an end user, containing prompts or additional context information."
    );
  export type User = z.infer<typeof UserSchema>;
  // export interface User {
  //   role: "user";
  //   content: RichContent;
  //   name?: string;
  // }

  export const ToolSchema = z
    .object({
      role: z.literal("tool"),
      content: RichContentSchema,
      tool_call_id: z
        .string()
        .describe(
          "The unique identifier for the tool call that this message is responding to."
        ),
    })
    .describe(
      "Messages sent by tools in response to tool calls made by the assistant."
    );
  export type Tool = z.infer<typeof ToolSchema>;
  // export interface Tool {
  //   role: "tool";
  //   content: RichContent;
  //   tool_call_id: string;
  // }

  export const AssistantSchema = z
    .object({
      role: z.literal("assistant"),
      content: RichContentSchema.optional().nullable(),
      name: NameSchema.optional().nullable(),
      refusal: z
        .string()
        .optional()
        .nullable()
        .describe("The refusal message by the assistant."),
      tool_calls: z.array(Assistant.ToolCallSchema).optional().nullable(),
      reasoning: z
        .string()
        .optional()
        .nullable()
        .describe("The reasoning provided by the assistant."),
    })
    .describe("Messages sent by the model in response to user messages.");
  export type Assistant = z.infer<typeof AssistantSchema>;
  // export interface Assistant {
  //   role: "assistant";
  //   content?: RichContent | null;
  //   name?: string | null;
  //   refusal?: string | null;
  //   tool_calls?: Assistant.ToolCall[] | null;
  //   reasoning?: string | null;
  // }

  export namespace Assistant {
    export const ToolCallSchema = z.union([ToolCall.FunctionSchema]);
    export type ToolCall = z.infer<typeof ToolCallSchema>;
    // export type ToolCall = ToolCall.Function;

    export namespace ToolCall {
      export const FunctionSchema = z.object({
        type: z.literal("function"),
        id: z.string().describe("The unique identifier for the tool call."),
        function: Function.DefinitionSchema,
      });
      export type Function = z.infer<typeof FunctionSchema>;
      // export interface Function {
      //   type: "function";
      //   id: string;
      //   function: Function.Definition;
      // }

      export namespace Function {
        export const DefinitionSchema = z.object({
          name: z.string().describe("The name of the function called."),
          arguments: z
            .string()
            .describe("The arguments passed to the function."),
        });
        export type Definition = z.infer<typeof DefinitionSchema>;
      }
    }
  }
}

// Ensemble LLM

export const EnsembleLlmBaseSchema = z
  .object({
    model: z.string().describe("The full ID of the LLM to use."),
    output_mode: EnsembleLlm.OutputModeSchema,
    synthetic_reasoning: z
      .boolean()
      .optional()
      .nullable()
      .describe(
        "For Vector Completions only, whether to use synthetic reasoning prior to voting. Works for any LLM, even those that do not have native reasoning capabilities."
      ),
    top_logprobs: z
      .int()
      .min(0)
      .max(20)
      .optional()
      .nullable()
      .describe(
        "For Vector Completions only, whether to use logprobs to make the vote probabilistic. This means that the LLM can vote for multiple keys based on their logprobabilities. Allows LLMs to express native uncertainty when voting."
      ),
    prefix_messages: z
      .array(MessageSchema)
      .optional()
      .nullable()
      .describe(
        "Messages to prepend to every prompt sent to this LLM. Useful for setting context or influencing behavior."
      ),
    suffix_messages: z
      .array(MessageSchema)
      .optional()
      .nullable()
      .describe(
        "Messages to append to every prompt sent to this LLM. Useful for setting context or influencing behavior."
      ),
    frequency_penalty: z
      .number()
      .min(-2.0)
      .max(2.0)
      .optional()
      .nullable()
      .describe(
        "This setting aims to control the repetition of tokens based on how often they appear in the input. It tries to use less frequently those tokens that appear more in the input, proportional to how frequently they occur. Token penalty scales with the number of occurrences. Negative values will encourage token reuse."
      ),
    logit_bias: z
      .record(z.string(), z.number())
      .optional()
      .nullable()
      .describe(
        "Accepts a JSON object that maps tokens (specified by their token ID in the tokenizer) to an associated bias value from -100 to 100. Mathematically, the bias is added to the logits generated by the model prior to sampling. The exact effect will vary per model, but values between -1 and 1 should decrease or increase likelihood of selection; values like -100 or 100 should result in a ban or exclusive selection of the relevant token."
      ),
    max_completion_tokens: z
      .int()
      .min(0)
      .max(2147483647)
      .optional()
      .nullable()
      .describe(
        "An upper bound for the number of tokens that can be generated for a completion, including visible output tokens and reasoning tokens."
      ),
    presence_penalty: z
      .number()
      .min(-2.0)
      .max(2.0)
      .optional()
      .nullable()
      .describe(
        "This setting aims to control the presence of tokens in the output. It tries to encourage the model to use tokens that are less present in the input, proportional to their presence in the input. Token presence scales with the number of occurrences. Negative values will encourage more diverse token usage."
      ),
    stop: EnsembleLlm.StopSchema.optional().nullable(),
    temperature: z
      .number()
      .min(0.0)
      .max(2.0)
      .optional()
      .nullable()
      .describe(
        "This setting influences the variety in the model’s responses. Lower values lead to more predictable and typical responses, while higher values encourage more diverse and less common responses. At 0, the model always gives the same response for a given input."
      ),
    top_p: z
      .number()
      .min(0.0)
      .max(1.0)
      .optional()
      .nullable()
      .describe(
        "This setting limits the model’s choices to a percentage of likely tokens: only the top tokens whose probabilities add up to P. A lower value makes the model’s responses more predictable, while the default setting allows for a full range of token choices. Think of it like a dynamic Top-K."
      ),
    max_tokens: z
      .int()
      .min(0)
      .max(2147483647)
      .optional()
      .nullable()
      .describe(
        "This sets the upper limit for the number of tokens the model can generate in response. It won’t produce more than this limit. The maximum value is the context length minus the prompt length."
      ),
    min_p: z
      .number()
      .min(0.0)
      .max(1.0)
      .optional()
      .nullable()
      .describe(
        "Represents the minimum probability for a token to be considered, relative to the probability of the most likely token. (The value changes depending on the confidence level of the most probable token.) If your Min-P is set to 0.1, that means it will only allow for tokens that are at least 1/10th as probable as the best possible option."
      ),
    provider: EnsembleLlm.ProviderSchema.optional().nullable(),
    reasoning: EnsembleLlm.ReasoningSchema.optional().nullable(),
    repetition_penalty: z
      .number()
      .min(0.0)
      .max(2.0)
      .optional()
      .nullable()
      .describe(
        "Helps to reduce the repetition of tokens from the input. A higher value makes the model less likely to repeat tokens, but too high a value can make the output less coherent (often with run-on sentences that lack small words). Token penalty scales based on original token’s probability."
      ),
    top_a: z
      .number()
      .min(0.0)
      .max(1.0)
      .optional()
      .nullable()
      .describe(
        "Consider only the top tokens with “sufficiently high” probabilities based on the probability of the most likely token. Think of it like a dynamic Top-P. A lower Top-A value focuses the choices based on the highest probability token but with a narrower scope. A higher Top-A value does not necessarily affect the creativity of the output, but rather refines the filtering process based on the maximum probability."
      ),
    top_k: z
      .int()
      .min(0)
      .max(2147483647)
      .optional()
      .nullable()
      .describe(
        "This limits the model’s choice of tokens at each step, making it choose from a smaller set. A value of 1 means the model will always pick the most likely next token, leading to predictable results. By default this setting is disabled, making the model to consider all choices."
      ),
    verbosity: EnsembleLlm.VerbositySchema.optional().nullable(),
  })
  .describe(
    "An LLM to be used within an Ensemble or standalone with Chat Completions."
  );
export type EnsembleLlmBase = z.infer<typeof EnsembleLlmBaseSchema>;
// export interface EnsembleLlmBase {
//   model: string;
//   output_mode: EnsembleLlm.OutputMode;
//   synthetic_reasoning?: boolean | null;
//   top_logprobs?: number | null;
//   prefix_messages?: Message[] | null;
//   suffix_messages?: Message[] | null;
//   frequency_penalty?: number | null;
//   logit_bias?: Record<string, number> | null;
//   max_completion_tokens?: number | null;
//   presence_penalty?: number | null;
//   stop?: EnsembleLlm.Stop | null;
//   temperature?: number | null;
//   top_p?: number | null;
//   max_tokens?: number | null;
//   min_p?: number | null;
//   provider?: EnsembleLlm.Provider | null;
//   reasoning?: EnsembleLlm.Reasoning | null;
//   repetition_penalty?: number | null;
//   top_a?: number | null;
//   top_k?: number | null;
//   verbosity?: EnsembleLlm.Verbosity | null;
// }

export const EnsembleLlmBaseWithFallbacksAndCountSchema =
  EnsembleLlmBaseSchema.extend({
    count: z
      .uint32()
      .min(1)
      .optional()
      .nullable()
      .describe(
        "A count greater than one effectively means that there are multiple instances of this LLM in an ensemble."
      ),
    fallbacks: z
      .array(EnsembleLlmBaseSchema)
      .optional()
      .nullable()
      .describe("A list of fallback LLMs to use if the primary LLM fails."),
  }).describe(
    "An LLM to be used within an Ensemble, including optional fallbacks and count."
  );
export type EnsembleLlmBaseWithFallbacksAndCount = z.infer<
  typeof EnsembleLlmBaseWithFallbacksAndCountSchema
>;
// export interface EnsembleLlmBaseWithFallbacksAndCount {
//   count?: number | null;
//   inner: EnsembleLlmBase;
//   fallbacks?: EnsembleLlmBase[] | null;
// }

export const EnsembleLlmSchema = EnsembleLlmBaseSchema.extend({
  id: z.string().describe("The unique identifier for the Ensemble LLM."),
}).describe(
  "An LLM to be used within an Ensemble or standalone with Chat Completions, including its unique identifier."
);
export type EnsembleLlm = z.infer<typeof EnsembleLlmSchema>;
// export interface EnsembleLlm extends EnsembleLlmBase {
//   id: string;
// }

export const EnsembleLlmWithFallbacksAndCountSchema = EnsembleLlmSchema.extend({
  count: EnsembleLlmBaseWithFallbacksAndCountSchema.shape.count,
  fallbacks: z
    .array(EnsembleLlmSchema)
    .optional()
    .nullable()
    .describe(
      EnsembleLlmBaseWithFallbacksAndCountSchema.shape.fallbacks.description!
    ),
}).describe(
  "An LLM to be used within an Ensemble, including its unique identifier, optional fallbacks, and count."
);
export type EnsembleLlmWithFallbacksAndCount = z.infer<
  typeof EnsembleLlmWithFallbacksAndCountSchema
>;
// export interface EnsembleLlmWithFallbacksAndCount {
//   count?: number | null;
//   inner: EnsembleLlm;
//   fallbacks?: EnsembleLlm[] | null;
// }

export namespace EnsembleLlm {
  export const OutputModeSchema = z
    .enum(["instruction", "json_schema", "tool_call"])
    .describe(
      'For Vector Completions only, specifies the LLM\'s voting output mode. For "instruction", the assistant is instructed to output a key. For "json_schema", the assistant is constrained to output a valid key using a JSON schema. For "tool_call", the assistant is instructed to output a tool call to select the key.'
    );
  export type OutputMode = z.infer<typeof OutputModeSchema>;
  // export type OutputMode = "instruction" | "json_schema" | "tool_call";

  export const StopSchema = z
    .union([
      z
        .string()
        .describe("Generation will stop when this string is generated."),
      z
        .array(z.string())
        .describe(
          "Generation will stop when any of these strings are generated."
        ),
    ])
    .describe(
      "The assistant will stop when any of the provided strings are generated."
    );
  export type Stop = z.infer<typeof StopSchema>;
  // export type Stop = string | string[];

  export const ProviderSchema = z
    .object({
      allow_fallbacks: z
        .boolean()
        .optional()
        .nullable()
        .describe(
          "Whether to allow fallback providers if the preferred provider is unavailable."
        ),
      require_parameters: z
        .boolean()
        .optional()
        .nullable()
        .describe(
          "Whether to require that the provider supports all specified parameters."
        ),
      order: z
        .array(z.string())
        .optional()
        .nullable()
        .describe(
          "An ordered list of provider names to use when selecting a provider for this model."
        ),
      only: z
        .array(z.string())
        .optional()
        .nullable()
        .describe(
          "A list of provider names to restrict selection to when selecting a provider for this model."
        ),
      ignore: z
        .array(z.string())
        .optional()
        .nullable()
        .describe(
          "A list of provider names to ignore when selecting a provider for this model."
        ),
      quantizations: z
        .array(Provider.QuantizationSchema)
        .optional()
        .nullable()
        .describe(
          "Specifies the quantizations to allow when selecting providers for this model."
        ),
    })
    .describe("Options for selecting the upstream provider of this model.");
  export type Provider = z.infer<typeof ProviderSchema>;
  // export interface Provider {
  //   allow_fallbacks?: boolean | null;
  //   require_parameters?: boolean | null;
  //   order?: string[] | null;
  //   only?: string[] | null;
  //   ignore?: string[] | null;
  //   quantizations?: Provider.Quantization[] | null;
  // }

  export namespace Provider {
    export const QuantizationSchema = z
      .enum([
        "int4",
        "int8",
        "fp4",
        "fp6",
        "fp8",
        "fp16",
        "bf16",
        "fp32",
        "unknown",
      ])
      .describe("An LLM quantization.");
    export type Quantization = z.infer<typeof QuantizationSchema>;
    // export type Quantization =
    //   | "int4"
    //   | "int8"
    //   | "fp4"
    //   | "fp6"
    //   | "fp8"
    //   | "fp16"
    //   | "bf16"
    //   | "fp32"
    //   | "unknown";
  }

  export const ReasoningSchema = z
    .object({
      enabled: z
        .boolean()
        .optional()
        .nullable()
        .describe("Enables or disables reasoning for supported models."),
      max_tokens: z
        .int()
        .min(0)
        .max(2147483647)
        .optional()
        .nullable()
        .describe(
          "The maximum number of tokens to use for reasoning in a response."
        ),
      effort: Reasoning.EffortSchema.optional().nullable(),
      summary_verbosity: Reasoning.SummaryVerbositySchema.optional().nullable(),
    })
    .optional()
    .nullable()
    .describe("Options for controlling reasoning behavior of the model.");
  export type Reasoning = z.infer<typeof ReasoningSchema>;
  // export interface Reasoning {
  //   enabled?: boolean | null;
  //   max_tokens?: number | null;
  //   effort?: Reasoning.Effort | null;
  //   summary_verbosity?: Reasoning.SummaryVerbosity | null;
  // }

  export namespace Reasoning {
    export const EffortSchema = z
      .enum(["none", "minimal", "low", "medium", "high", "xhigh"])
      .describe(
        "Constrains effort on reasoning for supported reasoning models. Reducing reasoning effort can result in faster responses and fewer tokens used on reasoning in a response."
      );
    export type Effort = z.infer<typeof EffortSchema>;
    // export type Effort =
    //   | "none"
    //   | "minimal"
    //   | "low"
    //   | "medium"
    //   | "high"
    //   | "xhigh";

    export const SummaryVerbositySchema = z
      .enum(["auto", "concise", "detailed"])
      .describe(
        "Controls the verbosity of the reasoning summary for supported reasoning models."
      );
    export type SummaryVerbosity = z.infer<typeof SummaryVerbositySchema>;
    // export type SummaryVerbosity = "auto" | "concise" | "detailed";
  }

  export const VerbositySchema = z
    .enum(["low", "medium", "high"])
    .describe(
      "Controls the verbosity and length of the model response. Lower values produce more concise responses, while higher values produce more detailed and comprehensive responses."
    );
  export type Verbosity = z.infer<typeof VerbositySchema>;
  // export type Verbosity = "low" | "medium" | "high";

  export const ListItemSchema = z.object({
    id: z.string().describe("The unique identifier for the Ensemble LLM."),
  });
  export type ListItem = z.infer<typeof ListItemSchema>;
  // export interface ListItem {
  //   id: string;
  // }

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/ensemble_llms", options);
    return response as { data: ListItem[] };
  }

  export const RetrieveItemSchema = EnsembleLlmSchema.extend({
    created: z
      .int()
      .describe(
        "The Unix timestamp (in seconds) when the Ensemble LLM was created."
      ),
  });
  export type RetrieveItem = z.infer<typeof RetrieveItemSchema>;
  // export interface RetrieveItem extends EnsembleLlm {
  //   created: number;
  // }

  export async function retrieve(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<RetrieveItem> {
    const response = await openai.get(`/ensemble_llms/${id}`, options);
    return response as RetrieveItem;
  }

  export const HistoricalUsageSchema = z.object({
    requests: z
      .int()
      .describe("The total number of requests made to this Ensemble LLM."),
    completion_tokens: z
      .int()
      .describe(
        "The total number of completion tokens generated by this Ensemble LLM."
      ),
    prompt_tokens: z
      .int()
      .describe("The total number of prompt tokens sent to this Ensemble LLM."),
    total_cost: z
      .number()
      .describe("The total cost incurred by using this Ensemble LLM."),
  });
  export type HistoricalUsage = z.infer<typeof HistoricalUsageSchema>;
  // export interface HistoricalUsage {
  //   requests: number;
  //   completion_tokens: number;
  //   prompt_tokens: number;
  //   total_cost: number;
  // }

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

export const EnsembleBaseSchema = z
  .object({
    llms: z
      .array(EnsembleLlmBaseWithFallbacksAndCountSchema)
      .describe("The list of LLMs that make up the ensemble."),
  })
  .describe("An ensemble of LLMs.");
export type EnsembleBase = z.infer<typeof EnsembleBaseSchema>;
// export interface EnsembleBase {
//   llms: EnsembleLlmBaseWithFallbacksAndCount[];
// }

export const EnsembleSchema = z
  .object({
    id: z.string().describe("The unique identifier for the Ensemble."),
    llms: z
      .array(EnsembleLlmWithFallbacksAndCountSchema)
      .describe(EnsembleBaseSchema.shape.llms.description!),
  })
  .describe("An ensemble of LLMs with a unique identifier.");
export type Ensemble = z.infer<typeof EnsembleSchema>;
// export interface Ensemble {
//   id: string;
//   llms: EnsembleLlmWithFallbacksAndCount[];
// }

export namespace Ensemble {
  export const ListItemSchema = z.object({
    id: z.string().describe("The unique identifier for the Ensemble."),
  });
  export type ListItem = z.infer<typeof ListItemSchema>;
  // export interface ListItem {
  //   id: string;
  // }

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/ensembles", options);
    return response as { data: ListItem[] };
  }

  export const RetrieveItemSchema = EnsembleSchema.extend({
    created: z
      .int()
      .describe(
        "The Unix timestamp (in seconds) when the Ensemble was created."
      ),
  });
  export type RetrieveItem = z.infer<typeof RetrieveItemSchema>;
  // export interface RetrieveItem extends Ensemble {
  //   created: number;
  // }

  export async function retrieve(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<RetrieveItem> {
    const response = await openai.get(`/ensembles/${id}`, options);
    return response as RetrieveItem;
  }

  export const HistoricalUsageSchema = z.object({
    requests: z
      .int()
      .describe("The total number of requests made to this Ensemble."),
    completion_tokens: z
      .int()
      .describe(
        "The total number of completion tokens generated by this Ensemble."
      ),
    prompt_tokens: z
      .int()
      .describe("The total number of prompt tokens sent to this Ensemble."),
    total_cost: z
      .number()
      .describe("The total cost incurred by using this Ensemble."),
  });
  export type HistoricalUsage = z.infer<typeof HistoricalUsageSchema>;
  // export interface HistoricalUsage {
  //   requests: number;
  //   completion_tokens: number;
  //   prompt_tokens: number;
  //   total_cost: number;
  // }

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
      export const ProviderSchema = z
        .object({
          data_collection: Provider.DataCollectionSchema.optional().nullable(),
          zdr: z
            .boolean()
            .optional()
            .nullable()
            .describe(
              "Whether to enforce Zero Data Retention (ZDR) policies when selecting providers."
            ),
          sort: Provider.SortSchema.optional().nullable(),
          max_price: Provider.MaxPriceSchema.optional().nullable(),
          preferred_min_throughput: z
            .number()
            .optional()
            .nullable()
            .describe("Preferred minimum throughput for the provider."),
          preferred_max_latency: z
            .number()
            .optional()
            .nullable()
            .describe("Preferred maximum latency for the provider."),
          min_throughput: z
            .number()
            .optional()
            .nullable()
            .describe("Minimum throughput for the provider."),
          max_latency: z
            .number()
            .optional()
            .nullable()
            .describe("Maximum latency for the provider."),
        })
        .describe(
          "Options for selecting the upstream provider of this completion."
        );
      export type Provider = z.infer<typeof ProviderSchema>;
      // export interface Provider {
      //   data_collection?: Provider.DataCollection | null;
      //   zdr?: boolean | null;
      //   sort?: Provider.Sort | null;
      //   max_price?: Provider.MaxPrice | null;
      //   preferred_min_throughput?: number | null;
      //   preferred_max_latency?: number | null;
      //   min_throughput?: number | null;
      //   max_latency?: number | null;
      // }

      export namespace Provider {
        export const DataCollectionSchema = z
          .enum(["allow", "deny"])
          .describe("Specifies whether to allow providers which collect data.");
        export type DataCollection = z.infer<typeof DataCollectionSchema>;
        // export type DataCollection = "allow" | "deny";

        export const SortSchema = z
          .enum(["price", "throughput", "latency"])
          .describe("Specifies the sorting strategy for provider selection.");
        export type Sort = z.infer<typeof SortSchema>;
        // export type Sort = "price" | "throughput" | "latency";

        export const MaxPriceSchema = z.object({
          prompt: z
            .number()
            .optional()
            .nullable()
            .describe("Maximum price for prompt tokens."),
          completion: z
            .number()
            .optional()
            .nullable()
            .describe("Maximum price for completion tokens."),
          image: z
            .number()
            .optional()
            .nullable()
            .describe("Maximum price for image generation."),
          audio: z
            .number()
            .optional()
            .nullable()
            .describe("Maximum price for audio generation."),
          request: z
            .number()
            .optional()
            .nullable()
            .describe("Maximum price per request."),
        });
        export type MaxPrice = z.infer<typeof MaxPriceSchema>;
        // export interface MaxPrice {
        //   prompt?: number | null;
        //   completion?: number | null;
        //   image?: number | null;
        //   audio?: number | null;
        //   request?: number | null;
        // }
      }

      export const ModelSchema = z
        .union([z.string(), EnsembleLlmBaseSchema])
        .describe(
          "The Ensemble LLM to use for this completion. May be a unique ID or an inline definition."
        );
      export type Model = z.infer<typeof ModelSchema>;
      // export type Model = string | EnsembleLlmBase;

      export const ResponseFormatSchema = z
        .union([
          ResponseFormat.TextSchema,
          ResponseFormat.JsonObjectSchema,
          ResponseFormat.JsonSchemaSchema,
          ResponseFormat.GrammarSchema,
          ResponseFormat.PythonSchema,
        ])
        .describe("The desired format of the model's response.");
      export type ResponseFormat = z.infer<typeof ResponseFormatSchema>;
      // export type ResponseFormat =
      //   | ResponseFormat.Text
      //   | ResponseFormat.JsonObject
      //   | ResponseFormat.JsonSchema
      //   | ResponseFormat.Grammar
      //   | ResponseFormat.Python;

      export namespace ResponseFormat {
        export const TextSchema = z
          .object({
            type: z.literal("text"),
          })
          .describe("The response will be arbitrary text.");
        export type Text = z.infer<typeof TextSchema>;
        // export interface Text {
        //   type: "text";
        // }

        export const JsonObjectSchema = z
          .object({
            type: z.literal("json_object"),
          })
          .describe("The response will be a JSON object.");
        export type JsonObject = z.infer<typeof JsonObjectSchema>;
        // export interface JsonObject {
        //   type: "json_object";
        // }

        export const JsonSchemaSchema = z
          .object({
            type: z.literal("json_schema"),
            json_schema: JsonSchema.JsonSchemaSchema,
          })
          .describe("The response will conform to the provided JSON schema.");
        export type JsonSchema = z.infer<typeof JsonSchemaSchema>;
        // export interface JsonSchema {
        //   type: "json_schema";
        //   json_schema: JsonSchema.JsonSchema;
        // }

        export namespace JsonSchema {
          export const JsonSchemaSchema = z
            .object({
              name: z.string().describe("The name of the JSON schema."),
              description: z
                .string()
                .optional()
                .nullable()
                .describe("The description of the JSON schema."),
              schema: z
                .any()
                .optional()
                .describe("The JSON schema definition."),
              strict: z
                .boolean()
                .optional()
                .nullable()
                .describe(
                  "Whether to enforce strict adherence to the JSON schema."
                ),
            })
            .describe(
              "A JSON schema definition for constraining model output."
            );
          export type JsonSchema = z.infer<typeof JsonSchemaSchema>;
          // export interface JsonSchema {
          //   name: string;
          //   description?: string | null;
          //   schema?: JsonValue;
          //   strict?: boolean | null;
          // }
        }

        export const GrammarSchema = z
          .object({
            type: z.literal("grammar"),
            grammar: z
              .string()
              .describe("The grammar definition to constrain the response."),
          })
          .describe(
            "The response will conform to the provided grammar definition."
          );
        export type Grammar = z.infer<typeof GrammarSchema>;
        // export interface Grammar {
        //   type: "grammar";
        //   grammar: string;
        // }

        export const PythonSchema = z
          .object({
            type: z.literal("python"),
          })
          .describe("The response will be Python code.");
        export type Python = z.infer<typeof PythonSchema>;
        // export interface Python {
        //   type: "python";
        // }
      }

      export const ToolChoiceSchema = z
        .union([
          z.literal("none"),
          z.literal("auto"),
          z.literal("required"),
          ToolChoice.FunctionSchema,
        ])
        .describe("Specifies tool call behavior for the assistant.");
      export type ToolChoice = z.infer<typeof ToolChoiceSchema>;
      // export type ToolChoice =
      //   | "none"
      //   | "auto"
      //   | "required"
      //   | ToolChoice.Function;

      export namespace ToolChoice {
        export const FunctionSchema = z
          .object({
            type: z.literal("function"),
            function: Function.FunctionSchema,
          })
          .describe("Specify a function for the assistant to call.");
        export type Function = z.infer<typeof FunctionSchema>;
        // export interface Function {
        //   type: "function";
        //   function: Function.Function;
        // }

        export namespace Function {
          export const FunctionSchema = z.object({
            name: z
              .string()
              .describe("The name of the function the assistant will call."),
          });
          export type Function = z.infer<typeof FunctionSchema>;
          // export interface Function {
          //   name: string;
          // }
        }
      }

      export const ToolSchema = z
        .union([Tool.FunctionSchema])
        .describe("A tool that the assistant can call.");
      export type Tool = z.infer<typeof ToolSchema>;
      // export type Tool = Tool.Function;

      export namespace Tool {
        export const FunctionSchema = z
          .object({
            type: z.literal("function"),
            function: Function.DefinitionSchema,
          })
          .describe("A function tool that the assistant can call.");
        export type Function = z.infer<typeof FunctionSchema>;
        // export interface Function {
        //   type: "function";
        //   function: Function.Definition;
        // }

        export namespace Function {
          export const DefinitionSchema = z
            .object({
              name: z.string().describe("The name of the function."),
              description: z
                .string()
                .optional()
                .nullable()
                .describe("The description of the function."),
              parameters: z
                .record(z.string(), z.any())
                .optional()
                .nullable()
                .describe(
                  "The JSON schema defining the parameters of the function."
                ),
              strict: z
                .boolean()
                .optional()
                .nullable()
                .describe(
                  "Whether to enforce strict adherence to the parameter schema."
                ),
            })
            .describe("The definition of a function tool.");
          export type Definition = z.infer<typeof DefinitionSchema>;
          // export interface Definition {
          //   name: string;
          //   description?: string | null;
          //   parameters?: { [key: string]: JsonValue } | null;
          //   strict?: boolean | null;
          // }
        }
      }

      export const PredictionSchema = z
        .object({
          type: z.literal("content"),
          content: Prediction.ContentSchema,
        })
        .describe(
          "Configuration for a Predicted Output, which can greatly improve response times when large parts of the model response are known ahead of time. This is most common when you are regenerating a file with only minor changes to most of the content."
        );
      export type Prediction = z.infer<typeof PredictionSchema>;
      // export interface Prediction {
      //   type: "content";
      //   content: Prediction.Content;
      // }

      export namespace Prediction {
        export const ContentSchema = z.union([
          z.string(),
          z.array(Content.PartSchema),
        ]);
        export type Content = z.infer<typeof ContentSchema>;
        // export type Content = string | Content.Part[];

        export namespace Content {
          export const PartSchema = z
            .object({
              type: z.literal("text"),
              text: z.string(),
            })
            .describe("A part of the predicted content.");
          export type Part = z.infer<typeof PartSchema>;
          // export interface Part {
          //   type: "text";
          //   text: string;
          // }
        }
      }

      export const SeedSchema = z
        .int()
        .describe(
          "If specified, upstream systems will make a best effort to sample deterministically, such that repeated requests with the same seed and parameters should return the same result."
        );

      export const BackoffMaxElapsedTimeSchema = z
        .uint32()
        .describe(
          "The maximum total time in milliseconds to spend on retries when a transient error occurs."
        );

      export const FirstChunkTimeoutSchema = z
        .uint32()
        .describe(
          "The maximum time in milliseconds to wait for the first chunk of a streaming response."
        );

      export const OtherChunkTimeoutSchema = z
        .uint32()
        .describe(
          "The maximum time in milliseconds to wait between subsequent chunks of a streaming response."
        );

      export const ChatCompletionCreateParamsBaseSchema = z
        .object({
          messages: z
            .array(MessageSchema)
            .describe("The prompt for the completion."),
          provider: ProviderSchema.optional().nullable(),
          model: ModelSchema,
          models: z
            .array(ModelSchema)
            .optional()
            .nullable()
            .describe(
              "Fallback Ensemble LLMs to use if the primary Ensemble LLM fails."
            ),
          top_logprobs: z
            .int()
            .min(0)
            .max(20)
            .optional()
            .nullable()
            .describe(
              "An integer between 0 and 20 specifying the number of most likely tokens to return at each token position, each with an associated log probability."
            ),
          response_format: ResponseFormatSchema.optional().nullable(),
          seed: SeedSchema.optional().nullable(),
          tool_choice: ToolChoiceSchema.optional().nullable(),
          tools: z
            .array(ToolSchema)
            .optional()
            .nullable()
            .describe("A list of available tools for the assistant."),
          parallel_tool_calls: z
            .boolean()
            .optional()
            .nullable()
            .describe(
              "Whether to allow the model to make multiple tool calls in parallel."
            ),
          prediction: PredictionSchema.optional().nullable(),
          backoff_max_elapsed_time:
            BackoffMaxElapsedTimeSchema.optional().nullable(),
          first_chunk_timeout: FirstChunkTimeoutSchema.optional().nullable(),
          other_chunk_timeout: OtherChunkTimeoutSchema.optional().nullable(),
        })
        .describe("Base parameters for creating a chat completion.");
      export type ChatCompletionCreateParamsBase = z.infer<
        typeof ChatCompletionCreateParamsBaseSchema
      >;
      // export interface ChatCompletionCreateParamsBase {
      //   messages: Message[];
      //   provider?: Provider | null;
      //   model: Model;
      //   models?: Model[] | null;
      //   top_logprobs?: number | null;
      //   response_format?: ResponseFormat | null;
      //   seed?: number | null;
      //   tool_choice?: ToolChoice | null;
      //   tools?: Tool[] | null;
      //   parallel_tool_calls?: boolean | null;
      //   prediction?: Prediction | null;
      //   backoff_max_elapsed_time?: number | null;
      //   first_chunk_timeout?: number | null;
      //   other_chunk_timeout?: number | null;
      // }

      export const StreamTrueSchema = z
        .literal(true)
        .describe("Whether to stream the response as a series of chunks.");

      export const ChatCompletionCreateParamsStreamingSchema =
        ChatCompletionCreateParamsBaseSchema.extend({
          stream: StreamTrueSchema,
        }).describe("Parameters for creating a streaming chat completion.");
      export type ChatCompletionCreateParamsStreaming = z.infer<
        typeof ChatCompletionCreateParamsStreamingSchema
      >;
      // export interface ChatCompletionCreateParamsStreaming
      //   extends ChatCompletionCreateParamsBase {
      //   stream: true;
      // }

      export const StreamFalseSchema = z
        .literal(false)
        .describe("Whether to stream the response as a series of chunks.");

      export const ChatCompletionCreateParamsNonStreamingSchema =
        ChatCompletionCreateParamsBaseSchema.extend({
          stream: StreamFalseSchema.optional().nullable(),
        }).describe("Parameters for creating a unary chat completion.");
      export type ChatCompletionCreateParamsNonStreaming = z.infer<
        typeof ChatCompletionCreateParamsNonStreamingSchema
      >;

      // export interface ChatCompletionCreateParamsNonStreaming
      //   extends ChatCompletionCreateParamsBase {
      //   stream?: false | null;
      // }

      export const ChatCompletionCreateParamsSchema = z
        .union([
          ChatCompletionCreateParamsStreamingSchema,
          ChatCompletionCreateParamsNonStreamingSchema,
        ])
        .describe("Parameters for creating a chat completion.");
      export type ChatCompletionCreateParams = z.infer<
        typeof ChatCompletionCreateParamsSchema
      >;
      // export type ChatCompletionCreateParams =
      //   | ChatCompletionCreateParamsStreaming
      //   | ChatCompletionCreateParamsNonStreaming;
    }

    export namespace Response {
      export const FinishReasonSchema = z
        .enum(["stop", "length", "tool_calls", "content_filter", "error"])
        .describe(
          "The reason why the assistant ceased to generate further tokens."
        );
      export type FinishReason = z.infer<typeof FinishReasonSchema>;
      // export type FinishReason =
      //   | "stop"
      //   | "length"
      //   | "tool_calls"
      //   | "content_filter"
      //   | "error";

      export const UsageSchema = z
        .object({
          completion_tokens: z
            .number()
            .describe("The number of tokens generated in the completion."),
          prompt_tokens: z
            .number()
            .describe("The number of tokens in the prompt."),
          total_tokens: z
            .number()
            .describe(
              "The total number of tokens used in the prompt or generated in the completion."
            ),
          completion_tokens_details:
            Usage.CompletionTokensDetailsSchema.optional(),
          prompt_tokens_details: Usage.PromptTokensDetailsSchema.optional(),
          cost: z
            .number()
            .describe("The cost in credits incurred for this completion."),
          cost_details: Usage.CostDetailsSchema.optional(),
          total_cost: z
            .number()
            .describe(
              "The total cost in credits incurred including upstream costs."
            ),
          cost_multiplier: z
            .number()
            .describe(
              "The cost multiplier applied to upstream costs for computing ObjectiveAI costs."
            ),
          is_byok: z
            .boolean()
            .describe(
              "Whether the completion used a BYOK (Bring Your Own Key) API Key."
            ),
        })
        .describe("Token and cost usage statistics for the completion.");
      export type Usage = z.infer<typeof UsageSchema>;
      // export interface Usage {
      //   completion_tokens: number;
      //   prompt_tokens: number;
      //   total_tokens: number;
      //   completion_tokens_details?: Usage.CompletionTokensDetails;
      //   prompt_tokens_details?: Usage.PromptTokensDetails;
      //   cost: number;
      //   cost_details?: Usage.CostDetails;
      //   total_cost: number;
      //   cost_multiplier: number;
      //   is_byok: boolean;
      // }

      export namespace Usage {
        export const CompletionTokensDetailsSchema = z
          .object({
            accepted_prediction_tokens: z
              .number()
              .optional()
              .describe(
                "The number of accepted prediction tokens in the completion."
              ),
            audio_tokens: z
              .number()
              .optional()
              .describe(
                "The number of generated audio tokens in the completion."
              ),
            reasoning_tokens: z
              .number()
              .optional()
              .describe(
                "The number of generated reasoning tokens in the completion."
              ),
            rejected_prediction_tokens: z
              .number()
              .optional()
              .describe(
                "The number of rejected prediction tokens in the completion."
              ),
          })
          .describe("Detailed breakdown of generated completion tokens.");
        export type CompletionTokensDetails = z.infer<
          typeof CompletionTokensDetailsSchema
        >;

        // export interface CompletionTokensDetails {
        //   accepted_prediction_tokens?: number;
        //   audio_tokens?: number;
        //   reasoning_tokens?: number;
        //   rejected_prediction_tokens?: number;
        // }

        export const PromptTokensDetailsSchema = z
          .object({
            audio_tokens: z
              .number()
              .optional()
              .describe("The number of audio tokens in the prompt."),
            cached_tokens: z
              .number()
              .optional()
              .describe("The number of cached tokens in the prompt."),
            cache_write_tokens: z
              .number()
              .optional()
              .describe("The number of prompt tokens written to cache."),
            video_tokens: z
              .number()
              .optional()
              .describe("The number of video tokens in the prompt."),
          })
          .describe("Detailed breakdown of prompt tokens.");
        export type PromptTokensDetails = z.infer<
          typeof PromptTokensDetailsSchema
        >;
        // export interface PromptTokensDetails {
        //   audio_tokens?: number;
        //   cached_tokens?: number;
        //   cache_write_tokens?: number;
        //   video_tokens?: number;
        // }

        export const CostDetailsSchema = z
          .object({
            upstream_inference_cost: z
              .number()
              .optional()
              .describe("The cost incurred upstream."),
            upstream_upstream_inference_cost: z
              .number()
              .optional()
              .describe("The cost incurred by upstream's upstream."),
          })
          .describe("Detailed breakdown of upstream costs incurred.");
        export type CostDetails = z.infer<typeof CostDetailsSchema>;
        // export interface CostDetails {
        //   upstream_inference_cost?: number;
        //   upstream_upstream_inference_cost?: number;
        // }
      }

      export const LogprobsSchema = z
        .object({
          content: z
            .array(Logprobs.LogprobSchema)
            .optional()
            .nullable()
            .describe("The log probabilities of the tokens in the content."),
          refusal: z
            .array(Logprobs.LogprobSchema)
            .optional()
            .nullable()
            .describe("The log probabilities of the tokens in the refusal."),
        })
        .describe(
          "The log probabilities of the tokens generated by the model."
        );
      export type Logprobs = z.infer<typeof LogprobsSchema>;
      // export interface Logprobs {
      //   content: Logprobs.Logprob[] | null;
      //   refusal: Logprobs.Logprob[] | null;
      // }

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

        export const LogprobSchema = z
          .object({
            token: z
              .string()
              .describe("The token string which was selected by the sampler."),
            bytes: z
              .array(z.number())
              .optional()
              .nullable()
              .describe(
                "The byte representation of the token which was selected by the sampler."
              ),
            logprob: z
              .number()
              .describe(
                "The log probability of the token which was selected by the sampler."
              ),
            top_logprobs: z
              .array(Logprob.TopLogprobSchema)
              .describe(
                "The log probabilities of the top tokens for this position."
              ),
          })
          .describe(
            "The token which was selected by the sampler for this position as well as the logprobabilities of the top options."
          );
        export type Logprob = z.infer<typeof LogprobSchema>;
        // export interface Logprob {
        //   token: string;
        //   bytes: number[] | null;
        //   logprob: number;
        //   top_logprobs: Logprob.TopLogprob[];
        // }

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

          export const TopLogprobSchema = z
            .object({
              token: z.string().describe("The token string."),
              bytes: z
                .array(z.number())
                .optional()
                .nullable()
                .describe("The byte representation of the token."),
              logprob: z
                .number()
                .optional()
                .nullable()
                .describe("The log probability of the token."),
            })
            .describe(
              "The log probability of a token in the list of top tokens."
            );
          export type TopLogprob = z.infer<typeof TopLogprobSchema>;
          // export interface TopLogprob {
          //   token: string;
          //   bytes: number[] | null;
          //   logprob: number | null;
          // }
        }
      }

      export const RoleSchema = z
        .enum(["assistant"])
        .describe("The role of the message author.");
      export type Role = z.infer<typeof RoleSchema>;
      // export type Role = "assistant";

      export const ImageSchema = z
        .union([Image.ImageUrlSchema])
        .describe("An image generated by the model.");
      export type Image = z.infer<typeof ImageSchema>;
      // export type Image = Image.ImageUrl;

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

        export const ImageUrlSchema = z.object({
          type: z.literal("image_url"),
          image_url: z.object({
            url: z.string().describe("The Base64 URL of the generated image."),
          }),
        });
        export type ImageUrl = z.infer<typeof ImageUrlSchema>;
        // export interface ImageUrl {
        //   type: "image_url";
        //   image_url: { url: string };
        // }
      }

      export namespace Streaming {
        export const ToolCallSchema = z
          .union([ToolCall.FunctionSchema])
          .describe("A tool call made by the assistant.");
        export type ToolCall = z.infer<typeof ToolCallSchema>;
        // export type ToolCall = ToolCall.Function;

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

          export const FunctionSchema = z
            .object({
              index: z
                .int()
                .describe(
                  "The index of the tool call in the sequence of tool calls."
                ),
              type: z.literal("function").optional(),
              id: z
                .string()
                .optional()
                .describe("The unique identifier of the function tool."),
              function: Function.DefinitionSchema.optional(),
            })
            .describe("A function tool call made by the assistant.");
          export type Function = z.infer<typeof FunctionSchema>;
          // export interface Function {
          //   index: number;
          //   type?: "function";
          //   id?: string;
          //   function?: Function.Definition;
          // }

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

            export const DefinitionSchema = z.object({
              name: z.string().optional().describe("The name of the function."),
              arguments: z
                .string()
                .optional()
                .describe("The arguments passed to the function."),
            });
            export type Definition = z.infer<typeof DefinitionSchema>;
            // export interface Definition {
            //   name?: string;
            //   arguments?: string;
            // }

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

        export const DeltaSchema = z
          .object({
            content: z
              .string()
              .optional()
              .describe("The content added in this delta."),
            refusal: z
              .string()
              .optional()
              .describe("The refusal message added in this delta."),
            role: RoleSchema.optional(),
            tool_calls: z
              .array(ToolCallSchema)
              .optional()
              .describe("Tool calls made in this delta."),
            reasoning: z
              .string()
              .optional()
              .describe("The reasoning added in this delta."),
            images: z
              .array(ImageSchema)
              .optional()
              .describe("Images added in this delta."),
          })
          .describe("A delta in a streaming chat completion response.");
        export type Delta = z.infer<typeof DeltaSchema>;
        // export interface Delta {
        //   content?: string;
        //   refusal?: string;
        //   role?: Role;
        //   tool_calls?: ToolCall[];
        //   reasoning?: string;
        //   images?: Image[];
        // }

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

        export const ChoiceSchema = z
          .object({
            delta: DeltaSchema,
            finish_reason: FinishReasonSchema.optional(),
            index: z
              .int()
              .describe("The index of the choice in the list of choices."),
            logprobs: LogprobsSchema.optional(),
          })
          .describe("A choice in a streaming chat completion response.");
        export type Choice = z.infer<typeof ChoiceSchema>;
        // export interface Choice {
        //   delta: Delta;
        //   finish_reason: FinishReason | null;
        //   index: number;
        //   logprobs?: Logprobs;
        // }

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

        export const ChatCompletionChunkSchema = z
          .object({
            id: z
              .string()
              .describe("The unique identifier of the chat completion."),
            upstream_id: z
              .string()
              .describe(
                "The unique identifier of the upstream chat completion."
              ),
            choices: z
              .array(ChoiceSchema)
              .describe("The list of choices in this chunk."),
            created: z
              .number()
              .describe(
                "The Unix timestamp (in seconds) when the chat completion was created."
              ),
            model: z
              .string()
              .describe(
                "The unique identifier of the Ensemble LLM used for this chat completion."
              ),
            upstream_model: z
              .string()
              .describe("The upstream model used for this chat completion."),
            object: z.literal("chat.completion.chunk"),
            service_tier: z.string().optional(),
            system_fingerprint: z.string().optional(),
            usage: UsageSchema.optional(),
            provider: z
              .string()
              .optional()
              .describe("The provider used for this chat completion."),
          })
          .describe("A chunk in a streaming chat completion response.");
        export type ChatCompletionChunk = z.infer<
          typeof ChatCompletionChunkSchema
        >;
        // export interface ChatCompletionChunk {
        //   id: string;
        //   upstream_id: string;
        //   choices: Choice[];
        //   created: number;
        //   model: string;
        //   upstream_model: string;
        //   object: "chat.completion.chunk";
        //   service_tier?: string;
        //   system_fingerprint?: string;
        //   usage?: Usage;
        //   provider?: string;
        // }

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
      }

      export namespace Unary {
        export const ToolCallSchema = z
          .union([ToolCall.FunctionSchema])
          .describe(Streaming.ToolCallSchema.description!);
        export type ToolCall = z.infer<typeof ToolCallSchema>;
        // export type ToolCall = ToolCall.Function;

        export namespace ToolCall {
          export const FunctionSchema = z
            .object({
              type: z.literal("function"),
              id: z
                .string()
                .describe(
                  Streaming.ToolCall.FunctionSchema.shape.id.description!
                ),
              function: Function.DefinitionSchema,
            })
            .describe(Streaming.ToolCall.FunctionSchema.description!);
          export type Function = z.infer<typeof FunctionSchema>;
          // export interface Function {
          //   type: "function";
          //   id: string;
          //   function: Function.Definition;
          // }

          export namespace Function {
            // export interface Definition {
            //   name: string;
            //   arguments: string;
            // }
            export const DefinitionSchema = z.object({
              name: z
                .string()
                .describe(
                  Streaming.ToolCall.Function.DefinitionSchema.shape.name
                    .description!
                ),
              arguments: z
                .string()
                .describe(
                  Streaming.ToolCall.Function.DefinitionSchema.shape.arguments
                    .description!
                ),
            });
            export type Definition = z.infer<typeof DefinitionSchema>;
          }
        }

        export const MessageSchema = z
          .object({
            content: z
              .string()
              .nullable()
              .describe("The content of the message."),
            refusal: z
              .string()
              .nullable()
              .describe("The refusal message, if any."),
            role: RoleSchema,
            tool_calls: z
              .array(ToolCallSchema)
              .nullable()
              .describe("The tool calls made by the assistant, if any."),
            reasoning: z
              .string()
              .optional()
              .describe("The reasoning provided by the assistant, if any."),
            images: z
              .array(ImageSchema)
              .optional()
              .describe("The images generated by the assistant, if any."),
          })
          .describe("A message generated by the assistant.");
        // export interface Message {
        //   content: string | null;
        //   refusal: string | null;
        //   role: Role;
        //   tool_calls: ToolCall[] | null;
        //   reasoning?: string;
        //   images?: Image[];
        // }

        export const ChoiceSchema = z
          .object({
            message: MessageSchema,
            finish_reason: FinishReasonSchema,
            index: z
              .int()
              .describe(Streaming.ChoiceSchema.shape.index.description!),
            logprobs: LogprobsSchema.nullable(),
          })
          .describe("A choice in a unary chat completion response.");
        export type Choice = z.infer<typeof ChoiceSchema>;
        // export interface Choice {
        //   message: Message;
        //   finish_reason: FinishReason;
        //   index: number;
        //   logprobs: Logprobs | null;
        // }

        export const ChatCompletionSchema = z
          .object({
            id: z
              .string()
              .describe("The unique identifier of the chat completion."),
            upstream_id: z
              .string()
              .describe(
                "The unique identifier of the upstream chat completion."
              ),
            choices: z
              .array(ChoiceSchema)
              .describe("The list of choices in this chat completion."),
            created: z
              .number()
              .describe(
                "The Unix timestamp (in seconds) when the chat completion was created."
              ),
            model: z
              .string()
              .describe(
                "The unique identifier of the Ensemble LLM used for this chat completion."
              ),
            upstream_model: z
              .string()
              .describe("The upstream model used for this chat completion."),
            object: z.literal("chat.completion"),
            service_tier: z.string().optional(),
            system_fingerprint: z.string().optional(),
            usage: UsageSchema,
            provider: z
              .string()
              .optional()
              .describe("The provider used for this chat completion."),
          })
          .describe("A unary chat completion response.");
        export type ChatCompletion = z.infer<typeof ChatCompletionSchema>;
        // export interface ChatCompletion {
        //   id: string;
        //   upstream_id: string;
        //   choices: Choice[];
        //   created: number;
        //   model: string;
        //   upstream_model: string;
        //   object: "chat.completion";
        //   service_tier?: string;
        //   system_fingerprint?: string;
        //   usage: Usage;
        //   provider?: string;
        // }
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
      export const EnsembleSchema = z
        .union([z.string(), EnsembleBaseSchema])
        .describe(
          "The Ensemble to use for this completion. May be a unique ID or an inline definition."
        );
      export type Ensemble = z.infer<typeof EnsembleSchema>;
      // export type Ensemble = string | EnsembleBase;

      export const VectorCompletionCreateParamsBaseSchema = z
        .object({
          retry: z
            .string()
            .optional()
            .nullable()
            .describe(
              "A retry token from a previous incomplete or failed completion."
            ),
          messages: z
            .array(MessageSchema)
            .describe("The prompt for the completion."),
          provider:
            Chat.Completions.Request.ProviderSchema.optional().nullable(),
          ensemble: EnsembleSchema,
          profile: z
            .array(z.number())
            .describe(
              'The profile to use for the completion. Must be of the same length as the Ensemble\'s "LLMs" field, ignoring count.'
            ),
          seed: Chat.Completions.Request.SeedSchema.optional().nullable(),
          tools: z
            .array(Chat.Completions.Request.ToolSchema)
            .optional()
            .nullable()
            .describe(
              "A list of available tools for the assistant. These are readonly and will only be useful for explaining prior tool calls or otherwise influencing behavior."
            ),
          responses: z
            .array(Message.RichContentSchema)
            .describe(
              "A list of possible assistant responses which the ensemble will vote on. The output scores will be of the same length. Each corresponds to one response. The winner is the response with the highest score."
            ),
          backoff_max_elapsed_time:
            Chat.Completions.Request.BackoffMaxElapsedTimeSchema.optional().nullable(),
          first_chunk_timeout:
            Chat.Completions.Request.FirstChunkTimeoutSchema.optional().nullable(),
          other_chunk_timeout:
            Chat.Completions.Request.OtherChunkTimeoutSchema.optional().nullable(),
        })
        .describe("Base parameters for creating a vector completion.");
      export type VectorCompletionCreateParamsBase = z.infer<
        typeof VectorCompletionCreateParamsBaseSchema
      >;
      // export interface VectorCompletionCreateParamsBase {
      //   retry?: string | null;
      //   messages: Message[];
      //   provider?: Chat.Completions.Request.Provider | null;
      //   ensemble: Ensemble;
      //   profile: number[];
      //   seed?: number | null;
      //   tools?: Chat.Completions.Request.Tool[] | null;
      //   responses: Message.RichContent[];
      //   backoff_max_elapsed_time?: number | null;
      //   first_chunk_timeout?: number | null;
      //   other_chunk_timeout?: number | null;
      // }

      export const VectorCompletionCreateParamsStreamingSchema = z
        .object({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
        .describe("Parameters for creating a streaming vector completion.");
      export type VectorCompletionCreateParamsStreaming = z.infer<
        typeof VectorCompletionCreateParamsStreamingSchema
      >;
      // export interface VectorCompletionCreateParamsStreaming
      //   extends VectorCompletionCreateParamsBase {
      //   stream: true;
      // }

      export const VectorCompletionCreateParamsNonStreamingSchema = z
        .object({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
        .describe("Parameters for creating a unary vector completion.");
      export type VectorCompletionCreateParamsNonStreaming = z.infer<
        typeof VectorCompletionCreateParamsNonStreamingSchema
      >;

      // export interface VectorCompletionCreateParamsNonStreaming
      //   extends VectorCompletionCreateParamsBase {
      //   stream?: false | null;
      // }

      export const VectorCompletionCreateParamsSchema = z
        .union([
          VectorCompletionCreateParamsStreamingSchema,
          VectorCompletionCreateParamsNonStreamingSchema,
        ])
        .describe("Parameters for creating a vector completion.");
      export type VectorCompletionCreateParams = z.infer<
        typeof VectorCompletionCreateParamsSchema
      >;
      // export type VectorCompletionCreateParams =
      //   | VectorCompletionCreateParamsStreaming
      //   | VectorCompletionCreateParamsNonStreaming;
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
