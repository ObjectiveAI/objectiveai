import OpenAI from "openai";
import { Stream } from "openai/streaming";
import z from "zod";

// Expressions

export const ExpressionSchema = z
  .object({
    $jmespath: z.string().describe("A JMESPath expression."),
  })
  .describe("An expression which evaluates to a value.")
  .meta({ title: "Expression" });
export type Expression = z.infer<typeof ExpressionSchema>;

// JSON Value

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };
export const JsonValueSchema: z.ZodType<JsonValue> = z
  .lazy(() =>
    z.union([
      z.null(),
      z.boolean(),
      z.number(),
      z.string(),
      z.array(
        JsonValueSchema.meta({
          title: "JsonValue",
          recursive: true,
        })
      ),
      z.record(
        z.string(),
        JsonValueSchema.meta({
          title: "JsonValue",
          recursive: true,
        })
      ),
    ])
  )
  .describe("A JSON value.")
  .meta({ title: "JsonValue" });

export type JsonValueExpression =
  | null
  | boolean
  | number
  | string
  | (Expression | JsonValueExpression)[]
  | { [key: string]: Expression | JsonValueExpression }
  | Expression;
export const JsonValueExpressionSchema: z.ZodType<JsonValueExpression> = z
  .lazy(() =>
    z.union([
      z.null(),
      z.boolean(),
      z.number(),
      z.string(),
      z.array(
        JsonValueExpressionSchema.meta({
          title: "JsonValueExpression",
          recursive: true,
        })
      ),
      z.record(
        z.string(),
        JsonValueExpressionSchema.meta({
          title: "JsonValueExpression",
          recursive: true,
        })
      ),
      ExpressionSchema.describe(
        "An expression which evaluates to a JSON value."
      ),
    ])
  )
  .describe(JsonValueSchema.description!)
  .meta({ title: "JsonValueExpression" });

// Errors

export const ObjectiveAIErrorSchema = z
  .object({
    code: z.uint32().describe("The status code of the error."),
    message: z.any().describe("The message or details of the error."),
  })
  .describe("An error returned by the ObjectiveAI API.")
  .meta({ title: "ObjectiveAIError" });
export type ObjectiveAIError = z.infer<typeof ObjectiveAIErrorSchema>;

// Messages

export namespace Message {
  export namespace SimpleContent {
    export const TextSchema = z
      .string()
      .describe("Plain text content.")
      .meta({ title: "SimpleContentText" });
    export type Text = z.infer<typeof TextSchema>;

    export const PartSchema = z
      .object({
        type: z.literal("text"),
        text: z.string().describe("The text content."),
      })
      .describe("A simple content part.")
      .meta({ title: "SimpleContentPart" });
    export type Part = z.infer<typeof PartSchema>;

    export const PartExpressionSchema = z
      .union([
        PartSchema,
        ExpressionSchema.describe(
          "An expression which evaluates to a simple content part."
        ),
      ])
      .describe(PartSchema.description!)
      .meta({ title: "SimpleContentPartExpression" });
    export type PartExpression = z.infer<typeof PartExpressionSchema>;

    export const PartsSchema = z
      .array(PartSchema)
      .describe("An array of simple content parts.")
      .meta({ title: "SimpleContentParts" });
    export type Parts = z.infer<typeof PartsSchema>;

    export const PartsExpressionSchema = z
      .array(PartExpressionSchema)
      .describe(PartsSchema.description!)
      .meta({ title: "SimpleContentPartExpressions" });
    export type PartsExpression = z.infer<typeof PartsExpressionSchema>;
  }

  export const SimpleContentSchema = z
    .union([SimpleContent.TextSchema, SimpleContent.PartsSchema])
    .describe("Simple content.")
    .meta({ title: "SimpleContent" });
  export type SimpleContent = z.infer<typeof SimpleContentSchema>;

  export const SimpleContentExpressionSchema = z
    .union([
      SimpleContent.TextSchema,
      SimpleContent.PartsExpressionSchema,
      ExpressionSchema.describe(
        "An expression which evaluates to simple content."
      ),
    ])
    .describe(SimpleContentSchema.description!)
    .meta({ title: "SimpleContentExpression" });
  export type SimpleContentExpression = z.infer<
    typeof SimpleContentExpressionSchema
  >;

  export namespace RichContent {
    export const TextSchema = z
      .string()
      .describe("Plain text content.")
      .meta({ title: "RichContentText" });
    export type Text = z.infer<typeof TextSchema>;

    export namespace Part {
      export namespace Text {
        export const TextSchema = z.string().describe("The text content.");
        export type Text = z.infer<typeof TextSchema>;
      }

      export const TextSchema = z
        .object({
          type: z.literal("text"),
          text: Text.TextSchema,
        })
        .describe("A text rich content part.")
        .meta({ title: "TextRichContentPart" });
      export type Text = z.infer<typeof TextSchema>;

      export namespace ImageUrl {
        export const DetailSchema = z
          .enum(["auto", "low", "high"])
          .describe("Specifies the detail level of the image.");
        export type Detail = z.infer<typeof DetailSchema>;

        export const UrlSchema = z
          .string()
          .describe(
            "Either a URL of the image or the base64 encoded image data."
          );
        export type Url = z.infer<typeof UrlSchema>;

        export const DefinitionSchema = z
          .object({
            url: UrlSchema,
            detail: DetailSchema.optional().nullable(),
          })
          .describe("The URL of the image and its optional detail level.");
        export type Definition = z.infer<typeof DefinitionSchema>;
      }

      export const ImageUrlSchema = z
        .object({
          type: z.literal("image_url"),
          image_url: ImageUrl.DefinitionSchema,
        })
        .describe("An image rich content part.")
        .meta({ title: "ImageRichContentPart" });
      export type ImageUrl = z.infer<typeof ImageUrlSchema>;

      export namespace InputAudio {
        export const FormatSchema = z
          .enum(["wav", "mp3"])
          .describe("The format of the encoded audio data.");
        export type Format = z.infer<typeof FormatSchema>;

        export const DataSchema = z
          .string()
          .describe("Base64 encoded audio data.");
        export type Data = z.infer<typeof DataSchema>;

        export const DefinitionSchema = z
          .object({
            data: DataSchema,
            format: FormatSchema,
          })
          .describe("The audio data and its format.");
        export type Definition = z.infer<typeof DefinitionSchema>;
      }

      export const InputAudioSchema = z
        .object({
          type: z.literal("input_audio"),
          input_audio: InputAudio.DefinitionSchema,
        })
        .describe("An audio rich content part.")
        .meta({ title: "AudioRichContentPart" });
      export type InputAudio = z.infer<typeof InputAudioSchema>;

      export namespace VideoUrl {
        export const UrlSchema = z.string().describe("URL of the video.");
        export type Url = z.infer<typeof UrlSchema>;

        export const DefinitionSchema = z.object({
          url: UrlSchema,
        });
        export type Definition = z.infer<typeof DefinitionSchema>;
      }

      export const VideoUrlSchema = z
        .object({
          type: z.enum(["video_url", "input_video"]),
          video_url: VideoUrl.DefinitionSchema,
        })
        .describe("A video rich content part.")
        .meta({ title: "VideoRichContentPart" });
      export type VideoUrl = z.infer<typeof VideoUrlSchema>;

      export namespace File {
        export const FileDataSchema = z
          .string()
          .describe(
            "The base64 encoded file data, used when passing the file to the model as a string."
          );
        export type FileData = z.infer<typeof FileDataSchema>;

        export const FileIdSchema = z
          .string()
          .describe("The ID of an uploaded file to use as input.");
        export type FileId = z.infer<typeof FileIdSchema>;

        export const FilenameSchema = z
          .string()
          .describe(
            "The name of the file, used when passing the file to the model as a string."
          );
        export type Filename = z.infer<typeof FilenameSchema>;

        export const FileUrlSchema = z
          .string()
          .describe(
            "The URL of the file, used when passing the file to the model as a URL."
          );
        export type FileUrl = z.infer<typeof FileUrlSchema>;

        export const DefinitionSchema = z
          .object({
            file_data: FileDataSchema.optional().nullable(),
            file_id: FileIdSchema.optional().nullable(),
            filename: FilenameSchema.optional().nullable(),
            file_url: FileUrlSchema.optional().nullable(),
          })
          .describe(
            "The file to be used as input, either as base64 data, an uploaded file ID, or a URL."
          );
        export type Definition = z.infer<typeof DefinitionSchema>;
      }

      export const FileSchema = z
        .object({
          type: z.literal("file"),
          file: File.DefinitionSchema,
        })
        .describe("A file rich content part.")
        .meta({ title: "FileRichContentPart" });
      export type File = z.infer<typeof FileSchema>;
    }

    export const PartSchema = z
      .discriminatedUnion("type", [
        Part.TextSchema,
        Part.ImageUrlSchema,
        Part.InputAudioSchema,
        Part.VideoUrlSchema,
        Part.FileSchema,
      ])
      .describe("A rich content part.")
      .meta({ title: "RichContentPart" });
    export type Part = z.infer<typeof PartSchema>;

    export const PartExpressionSchema = z
      .union([
        PartSchema,
        ExpressionSchema.describe(
          "An expression which evaluates to a rich content part."
        ),
      ])
      .describe(PartSchema.description!)
      .meta({ title: "RichContentPartExpression" });
    export type PartExpression = z.infer<typeof PartExpressionSchema>;

    export const PartsSchema = z
      .array(PartSchema)
      .describe("An array of rich content parts.")
      .meta({ title: "RichContentParts" });
    export type Parts = z.infer<typeof PartsSchema>;

    export const PartsExpressionSchema = z
      .array(PartExpressionSchema)
      .describe(PartsSchema.description!)
      .meta({ title: "RichContentPartExpressions" });
    export type PartsExpression = z.infer<typeof PartsExpressionSchema>;
  }

  export const RichContentSchema = z
    .union([RichContent.TextSchema, RichContent.PartsSchema])
    .describe("Rich content.")
    .meta({ title: "RichContent" });
  export type RichContent = z.infer<typeof RichContentSchema>;

  export const RichContentExpressionSchema = z
    .union([
      RichContent.TextSchema,
      RichContent.PartsExpressionSchema,
      ExpressionSchema.describe(
        "An expression which evaluates to rich content."
      ),
    ])
    .describe(RichContentSchema.description!)
    .meta({ title: "RichContentExpression" });
  export type RichContentExpression = z.infer<
    typeof RichContentExpressionSchema
  >;

  export const NameSchema = z
    .string()
    .describe(
      "An optional name for the participant. Provides the model information to differentiate between participants of the same role."
    )
    .meta({ title: "MessageName" });
  export type Name = z.infer<typeof NameSchema>;

  export const NameExpressionSchema = z
    .union([
      NameSchema,
      ExpressionSchema.describe("An expression which evaluates to a string."),
    ])
    .describe(NameSchema.description!)
    .meta({ title: "MessageNameExpression" });
  export type NameExpression = z.infer<typeof NameExpressionSchema>;

  export const DeveloperSchema = z
    .object({
      role: z.literal("developer"),
      content: SimpleContentSchema,
      name: NameSchema.optional().nullable(),
    })
    .describe(
      "Developer-provided instructions that the model should follow, regardless of messages sent by the user."
    )
    .meta({ title: "DeveloperMessage" });
  export type Developer = z.infer<typeof DeveloperSchema>;

  export const DeveloperExpressionSchema = z
    .object({
      role: z.literal("developer"),
      content: SimpleContentExpressionSchema,
      name: NameExpressionSchema.optional().nullable(),
    })
    .describe(DeveloperSchema.description!)
    .meta({ title: "DeveloperMessageExpression" });
  export type DeveloperExpression = z.infer<typeof DeveloperExpressionSchema>;

  export const SystemSchema = z
    .object({
      role: z.literal("system"),
      content: SimpleContentSchema,
      name: NameSchema.optional().nullable(),
    })
    .describe(
      "Developer-provided instructions that the model should follow, regardless of messages sent by the user."
    )
    .meta({ title: "SystemMessage" });
  export type System = z.infer<typeof SystemSchema>;

  export const SystemExpressionSchema = z
    .object({
      role: z.literal("system"),
      content: SimpleContentExpressionSchema,
      name: NameExpressionSchema.optional().nullable(),
    })
    .describe(SystemSchema.description!)
    .meta({ title: "SystemMessageExpression" });
  export type SystemExpression = z.infer<typeof SystemExpressionSchema>;

  export const UserSchema = z
    .object({
      role: z.literal("user"),
      content: RichContentSchema,
      name: NameSchema.optional().nullable(),
    })
    .describe(
      "Messages sent by an end user, containing prompts or additional context information."
    )
    .meta({ title: "UserMessage" });
  export type User = z.infer<typeof UserSchema>;

  export const UserExpressionSchema = z
    .object({
      role: z.literal("user"),
      content: RichContentExpressionSchema,
      name: NameExpressionSchema.optional().nullable(),
    })
    .describe(UserSchema.description!)
    .meta({ title: "UserMessageExpression" });
  export type UserExpression = z.infer<typeof UserExpressionSchema>;

  export namespace Tool {
    export const ToolCallIdSchema = z
      .string()
      .describe("The ID of the tool call that this message is responding to.")
      .meta({ title: "ToolMessageToolCallId" });
    export type ToolCallId = z.infer<typeof ToolCallIdSchema>;

    export const ToolCallIdExpressionSchema = z
      .union([
        ToolCallIdSchema,
        ExpressionSchema.describe("An expression which evaluates to a string."),
      ])
      .describe(ToolCallIdSchema.description!)
      .meta({ title: "ToolMessageToolCallIdExpression" });
    export type ToolCallIdExpression = z.infer<
      typeof ToolCallIdExpressionSchema
    >;
  }

  export const ToolSchema = z
    .object({
      role: z.literal("tool"),
      content: RichContentSchema,
      tool_call_id: Tool.ToolCallIdSchema,
    })
    .describe(
      "Messages sent by tools in response to tool calls made by the assistant."
    )
    .meta({ title: "ToolMessage" });
  export type Tool = z.infer<typeof ToolSchema>;

  export const ToolExpressionSchema = z
    .object({
      role: z.literal("tool"),
      content: RichContentExpressionSchema,
      tool_call_id: Tool.ToolCallIdExpressionSchema,
    })
    .describe(ToolSchema.description!)
    .meta({ title: "ToolMessageExpression" });
  export type ToolExpression = z.infer<typeof ToolExpressionSchema>;

  export namespace Assistant {
    export const RefusalSchema = z
      .string()
      .describe("The refusal message by the assistant.")
      .meta({ title: "AssistantMessageRefusal" });
    export type Refusal = z.infer<typeof RefusalSchema>;

    export const RefusalExpressionSchema = z
      .union([
        RefusalSchema,
        ExpressionSchema.describe("An expression which evaluates to a string."),
      ])
      .describe(RefusalSchema.description!)
      .meta({ title: "AssistantMessageRefusalExpression" });
    export type RefusalExpression = z.infer<typeof RefusalExpressionSchema>;

    export const ReasoningSchema = z
      .string()
      .describe("The reasoning provided by the assistant.")
      .meta({ title: "AssistantMessageReasoning" });
    export type Reasoning = z.infer<typeof ReasoningSchema>;

    export const ReasoningExpressionSchema = z
      .union([
        ReasoningSchema,
        ExpressionSchema.describe("An expression which evaluates to a string."),
      ])
      .describe(ReasoningSchema.description!)
      .meta({ title: "AssistantMessageReasoningExpression" });
    export type ReasoningExpression = z.infer<typeof ReasoningExpressionSchema>;

    export namespace ToolCall {
      export const IdSchema = z
        .string()
        .describe("The unique identifier for the tool call.")
        .meta({ title: "AssistantMessageToolCallId" });
      export type Id = z.infer<typeof IdSchema>;

      export const IdExpressionSchema = z
        .union([
          IdSchema,
          ExpressionSchema.describe(
            "An expression which evaluates to a string."
          ),
        ])
        .describe(IdSchema.description!)
        .meta({ title: "AssistantMessageToolCallIdExpression" });
      export type IdExpression = z.infer<typeof IdExpressionSchema>;

      export namespace Function {
        export const NameSchema = z
          .string()
          .describe("The name of the function called.")
          .meta({ title: "AssistantMessageToolCallFunctionName" });
        export type Name = z.infer<typeof NameSchema>;

        export const NameExpressionSchema = z
          .union([
            NameSchema,
            ExpressionSchema.describe(
              "An expression which evaluates to a string."
            ),
          ])
          .describe(NameSchema.description!)
          .meta({ title: "AssistantMessageToolCallFunctionNameExpression" });
        export type NameExpression = z.infer<typeof NameExpressionSchema>;

        export const ArgumentsSchema = z
          .string()
          .describe("The arguments passed to the function.")
          .meta({ title: "AssistantMessageToolCallFunctionArguments" });
        export type Arguments = z.infer<typeof ArgumentsSchema>;

        export const ArgumentsExpressionSchema = z
          .union([
            ArgumentsSchema,
            ExpressionSchema.describe(
              "An expression which evaluates to a string."
            ),
          ])
          .describe(ArgumentsSchema.description!)
          .meta({
            title: "AssistantMessageToolCallFunctionArgumentsExpression",
          });
        export type ArgumentsExpression = z.infer<
          typeof ArgumentsExpressionSchema
        >;

        export const DefinitionSchema = z
          .object({
            name: NameSchema,
            arguments: ArgumentsSchema,
          })
          .describe("The name and arguments of the function called.")
          .meta({ title: "AssistantMessageToolCallFunctionDefinition" });
        export type Definition = z.infer<typeof DefinitionSchema>;

        export const DefinitionExpressionSchema = z
          .object({
            name: NameExpressionSchema,
            arguments: ArgumentsExpressionSchema,
          })
          .describe(DefinitionSchema.description!)
          .meta({
            title: "AssistantMessageToolCallFunctionDefinitionExpression",
          });
        export type DefinitionExpression = z.infer<
          typeof DefinitionExpressionSchema
        >;
      }

      export const FunctionSchema = z
        .object({
          type: z.literal("function"),
          id: IdSchema,
          function: Function.DefinitionSchema,
        })
        .describe("A function tool call made by the assistant.")
        .meta({ title: "AssistantMessageToolCallFunction" });
      export type Function = z.infer<typeof FunctionSchema>;

      export const FunctionExpressionSchema = z
        .object({
          type: z.literal("function"),
          id: IdExpressionSchema,
          function: Function.DefinitionExpressionSchema,
        })
        .describe(FunctionSchema.description!)
        .meta({ title: "AssistantMessageToolCallFunctionExpression" });
      export type FunctionExpression = z.infer<typeof FunctionExpressionSchema>;
    }

    export const ToolCallSchema = z
      .union([ToolCall.FunctionSchema])
      .describe("A tool call made by the assistant.")
      .meta({ title: "AssistantMessageToolCall" });
    export type ToolCall = z.infer<typeof ToolCallSchema>;

    export const ToolCallExpressionSchema = z
      .union([
        ToolCall.FunctionExpressionSchema,
        ExpressionSchema.describe(
          "An expression which evaluates to a tool call."
        ),
      ])
      .describe(ToolCallSchema.description!)
      .meta({ title: "AssistantMessageToolCallExpression" });
    export type ToolCallExpression = z.infer<typeof ToolCallExpressionSchema>;

    export const ToolCallsSchema = z
      .array(ToolCallSchema)
      .describe("Tool calls made by the assistant.")
      .meta({ title: "AssistantMessageToolCalls" });
    export type ToolCalls = z.infer<typeof ToolCallsSchema>;

    export const ToolCallsExpressionSchema = z
      .union([
        z
          .array(ToolCallExpressionSchema)
          .describe(ToolCallsSchema.description!),
        ExpressionSchema.describe(
          "An expression which evaluates to an array of tool calls."
        ),
      ])
      .describe(ToolCallsSchema.description!)
      .meta({ title: "AssistantMessageToolCallsExpression" });
    export type ToolCallsExpression = z.infer<typeof ToolCallsExpressionSchema>;
  }

  export const AssistantSchema = z
    .object({
      role: z.literal("assistant"),
      content: RichContentSchema.optional().nullable(),
      name: NameSchema.optional().nullable(),
      refusal: Assistant.RefusalSchema.optional().nullable(),
      tool_calls: Assistant.ToolCallsSchema.optional().nullable(),
      reasoning: Assistant.ReasoningSchema.optional().nullable(),
    })
    .describe("Messages sent by the model in response to user messages.")
    .meta({ title: "AssistantMessage" });
  export type Assistant = z.infer<typeof AssistantSchema>;

  export const AssistantExpressionSchema = z
    .object({
      role: z.literal("assistant"),
      content: RichContentExpressionSchema.optional().nullable(),
      name: NameExpressionSchema.optional().nullable(),
      refusal: Assistant.RefusalExpressionSchema.optional().nullable(),
      tool_calls: Assistant.ToolCallsExpressionSchema.optional().nullable(),
      reasoning: Assistant.ReasoningExpressionSchema.optional().nullable(),
    })
    .describe(AssistantSchema.description!)
    .meta({ title: "AssistantMessageExpression" });
  export type AssistantExpression = z.infer<typeof AssistantExpressionSchema>;
}

export const MessageSchema = z
  .discriminatedUnion("role", [
    Message.DeveloperSchema,
    Message.SystemSchema,
    Message.UserSchema,
    Message.ToolSchema,
    Message.AssistantSchema,
  ])
  .describe("A message exchanged in a chat conversation.")
  .meta({ title: "Message" });
export type Message = z.infer<typeof MessageSchema>;

export const MessageExpressionSchema = z
  .union([
    z
      .discriminatedUnion("role", [
        Message.DeveloperExpressionSchema,
        Message.SystemExpressionSchema,
        Message.UserExpressionSchema,
        Message.ToolExpressionSchema,
        Message.AssistantExpressionSchema,
      ])
      .describe(MessageSchema.description!),
    ExpressionSchema.describe("An expression which evaluates to a message."),
  ])
  .describe(MessageSchema.description!)
  .meta({ title: "MessageExpression" });
export type MessageExpression = z.infer<typeof MessageExpressionSchema>;

export const MessagesSchema = z
  .array(MessageSchema)
  .describe("A list of messages exchanged in a chat conversation.")
  .meta({ title: "Messages" });
export type Messages = z.infer<typeof MessagesSchema>;

export const MessagesExpressionSchema = z
  .union([
    z
      .array(MessageExpressionSchema)
      .describe(MessagesSchema.description!)
      .meta({ title: "MessageExpressions" }),
    ExpressionSchema.describe(
      "An expression which evaluates to an array of messages."
    ),
  ])
  .describe(MessagesSchema.description!)
  .meta({ title: "MessagesExpression" });
export type MessagesExpression = z.infer<typeof MessagesExpressionSchema>;

// Tools

export namespace Tool {
  export namespace Function {
    export const NameSchema = z
      .string()
      .describe("The name of the function.")
      .meta({ title: "FunctionToolName" });
    export type Name = z.infer<typeof NameSchema>;

    export const NameExpressionSchema = z
      .union([
        NameSchema,
        ExpressionSchema.describe("An expression which evaluates to a string."),
      ])
      .describe(NameSchema.description!)
      .meta({ title: "FunctionToolNameExpression" });
    export type NameExpression = z.infer<typeof NameExpressionSchema>;

    export const DescriptionSchema = z
      .string()
      .describe("The description of the function.")
      .meta({ title: "FunctionToolDescription" });
    export type Description = z.infer<typeof DescriptionSchema>;

    export const DescriptionExpressionSchema = z
      .union([
        DescriptionSchema,
        ExpressionSchema.describe("An expression which evaluates to a string."),
      ])
      .describe(DescriptionSchema.description!)
      .meta({ title: "FunctionToolDescriptionExpression" });
    export type DescriptionExpression = z.infer<
      typeof DescriptionExpressionSchema
    >;

    export const ParametersSchema = z
      .record(z.string(), JsonValueSchema)
      .describe("The JSON schema defining the parameters of the function.")
      .meta({ title: "FunctionToolParameters" });
    export type Parameters = z.infer<typeof ParametersSchema>;

    export const ParametersExpressionSchema = z
      .union([
        z.record(z.string(), JsonValueExpressionSchema),
        ExpressionSchema.describe(
          "An expression which evaluates to a JSON schema object."
        ),
      ])
      .describe(ParametersSchema.description!)
      .meta({ title: "FunctionToolParametersExpression" });
    export type ParametersExpression = z.infer<
      typeof ParametersExpressionSchema
    >;

    export const StrictSchema = z
      .boolean()
      .describe("Whether to enforce strict adherence to the parameter schema.")
      .meta({ title: "FunctionToolStrict" });
    export type Strict = z.infer<typeof StrictSchema>;

    export const StrictExpressionSchema = z
      .union([
        StrictSchema,
        ExpressionSchema.describe(
          "An expression which evaluates to a boolean."
        ),
      ])
      .describe(StrictSchema.description!)
      .meta({ title: "FunctionToolStrictExpression" });
    export type StrictExpression = z.infer<typeof StrictExpressionSchema>;

    export const DefinitionSchema = z
      .object({
        name: NameSchema,
        description: DescriptionSchema.optional().nullable(),
        parameters: ParametersSchema.optional().nullable(),
        strict: StrictSchema.optional().nullable(),
      })
      .describe("The definition of a function tool.")
      .meta({ title: "FunctionToolDefinition" });
    export type Definition = z.infer<typeof DefinitionSchema>;

    export const DefinitionExpressionSchema = z
      .object({
        name: NameExpressionSchema,
        description: DescriptionExpressionSchema.optional().nullable(),
        parameters: ParametersExpressionSchema.optional().nullable(),
        strict: StrictExpressionSchema.optional().nullable(),
      })
      .describe(DefinitionSchema.description!)
      .meta({ title: "FunctionToolDefinitionExpression" });
    export type DefinitionExpression = z.infer<
      typeof DefinitionExpressionSchema
    >;
  }

  export const FunctionSchema = z
    .object({
      type: z.literal("function"),
      function: Function.DefinitionSchema,
    })
    .describe("A function tool that the assistant can call.")
    .meta({ title: "FunctionTool" });
  export type Function = z.infer<typeof FunctionSchema>;

  export const FunctionExpressionSchema = z
    .object({
      type: z.literal("function"),
      function: Function.DefinitionExpressionSchema,
    })
    .describe(FunctionSchema.description!)
    .meta({ title: "FunctionToolExpression" });
  export type FunctionExpression = z.infer<typeof FunctionExpressionSchema>;
}

export const ToolSchema = z
  .union([Tool.FunctionSchema])
  .describe("A tool that the assistant can call.")
  .meta({ title: "Tool" });
export type Tool = z.infer<typeof ToolSchema>;

export const ToolExpressionSchema = z
  .union([
    Tool.FunctionExpressionSchema,
    ExpressionSchema.describe("An expression which evaluates to a tool."),
  ])
  .describe(ToolSchema.description!)
  .meta({ title: "ToolExpression" });
export type ToolExpression = z.infer<typeof ToolExpressionSchema>;

export const ToolsSchema = z
  .array(ToolSchema)
  .describe("A list of tools that the assistant can call.")
  .meta({ title: "Tools" });
export type Tools = z.infer<typeof ToolsSchema>;

export const ToolsExpressionSchema = z
  .union([
    z
      .array(ToolExpressionSchema)
      .describe(ToolsSchema.description!)
      .meta({ title: "ToolExpressions" }),
    ExpressionSchema.describe(
      "An expression which evaluates to an array of tools."
    ),
  ])
  .describe(ToolsSchema.description!)
  .meta({ title: "ToolsExpression" });
export type ToolsExpression = z.infer<typeof ToolsExpressionSchema>;

// Vector Responses

export const VectorResponseSchema = Message.RichContentSchema.describe(
  "A possible assistant response. The LLMs in the Ensemble may vote for this option."
).meta({ title: "VectorResponse" });
export type VectorResponse = z.infer<typeof VectorResponseSchema>;

export const VectorResponseExpressionSchema = z
  .union([
    VectorResponseSchema,
    ExpressionSchema.describe(
      "An expression which evaluates to a possible assistant response."
    ),
  ])
  .describe(VectorResponseSchema.description!)
  .meta({ title: "VectorResponseExpression" });
export type VectorResponseExpression = z.infer<
  typeof VectorResponseExpressionSchema
>;

export const VectorResponsesSchema = z
  .array(VectorResponseSchema)
  .describe(
    "A list of possible assistant responses which the LLMs in the Ensemble will vote on. The output scores will be of the same length, each corresponding to one response. The winner is the response with the highest score."
  )
  .meta({ title: "VectorResponses" });
export type VectorResponses = z.infer<typeof VectorResponsesSchema>;

export const VectorResponsesExpressionSchema = z
  .union([
    z
      .array(VectorResponseExpressionSchema)
      .describe(VectorResponsesSchema.description!)
      .meta({ title: "VectorResponseExpressions" }),
    ExpressionSchema.describe(
      "An expression which evaluates to an array of possible assistant responses."
    ),
  ])
  .describe(VectorResponsesSchema.description!)
  .meta({ title: "VectorResponsesExpression" });
export type VectorResponsesExpression = z.infer<
  typeof VectorResponsesExpressionSchema
>;

// Ensemble LLM

export namespace EnsembleLlm {
  export const OutputModeSchema = z
    .enum(["instruction", "json_schema", "tool_call"])
    .describe(
      'For Vector Completions only, specifies the LLM\'s voting output mode. For "instruction", the assistant is instructed to output a key. For "json_schema", the assistant is constrained to output a valid key using a JSON schema. For "tool_call", the assistant is instructed to output a tool call to select the key.'
    );
  export type OutputMode = z.infer<typeof OutputModeSchema>;

  export const StopSchema = z
    .union([
      z
        .string()
        .describe("Generation will stop when this string is generated.")
        .meta({ title: "StopString" }),
      z
        .array(z.string().meta({ title: "StopString" }))
        .describe(
          "Generation will stop when any of these strings are generated."
        )
        .meta({ title: "StopStrings" }),
    ])
    .describe(
      "The assistant will stop when any of the provided strings are generated."
    )
    .meta({ title: "Stop" });
  export type Stop = z.infer<typeof StopSchema>;

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
      .describe("An LLM quantization.")
      .meta({ title: "ProviderQuantization" });
    export type Quantization = z.infer<typeof QuantizationSchema>;
  }

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
        .array(z.string().meta({ title: "ProviderName" }))
        .optional()
        .nullable()
        .describe(
          "An ordered list of provider names to use when selecting a provider for this model."
        ),
      only: z
        .array(z.string().meta({ title: "ProviderName" }))
        .optional()
        .nullable()
        .describe(
          "A list of provider names to restrict selection to when selecting a provider for this model."
        ),
      ignore: z
        .array(z.string().meta({ title: "ProviderName" }))
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

  export namespace Reasoning {
    export const EffortSchema = z
      .enum(["none", "minimal", "low", "medium", "high", "xhigh"])
      .describe(
        "Constrains effort on reasoning for supported reasoning models. Reducing reasoning effort can result in faster responses and fewer tokens used on reasoning in a response."
      )
      .meta({ title: "ReasoningEffort" });
    export type Effort = z.infer<typeof EffortSchema>;

    export const SummaryVerbositySchema = z
      .enum(["auto", "concise", "detailed"])
      .describe(
        "Controls the verbosity of the reasoning summary for supported reasoning models."
      )
      .meta({ title: "ReasoningSummaryVerbosity" });
    export type SummaryVerbosity = z.infer<typeof SummaryVerbositySchema>;
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

  export const VerbositySchema = z
    .enum(["low", "medium", "high"])
    .describe(
      "Controls the verbosity and length of the model response. Lower values produce more concise responses, while higher values produce more detailed and comprehensive responses."
    );
  export type Verbosity = z.infer<typeof VerbositySchema>;

  export const ListItemSchema = z.object({
    id: z.string().describe("The unique identifier for the Ensemble LLM."),
  });
  export type ListItem = z.infer<typeof ListItemSchema>;

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/ensemble_llms", options);
    return response as { data: ListItem[] };
  }

  export const RetrieveItemSchema = z.lazy(() =>
    EnsembleLlmSchema.extend({
      created: z
        .uint32()
        .describe(
          "The Unix timestamp (in seconds) when the Ensemble LLM was created."
        ),
    })
  );
  export type RetrieveItem = z.infer<typeof RetrieveItemSchema>;

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
      .uint32()
      .describe("The total number of requests made to this Ensemble LLM."),
    completion_tokens: z
      .uint32()
      .describe(
        "The total number of completion tokens generated by this Ensemble LLM."
      ),
    prompt_tokens: z
      .uint32()
      .describe("The total number of prompt tokens sent to this Ensemble LLM."),
    total_cost: z
      .number()
      .describe("The total cost incurred by using this Ensemble LLM."),
  });
  export type HistoricalUsage = z.infer<typeof HistoricalUsageSchema>;

  export async function retrieveUsage(
    openai: OpenAI,
    id: string,
    options?: OpenAI.RequestOptions
  ): Promise<HistoricalUsage> {
    const response = await openai.get(`/ensemble_llms/${id}/usage`, options);
    return response as HistoricalUsage;
  }
}

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
    prefix_messages: MessagesSchema.optional()
      .nullable()
      .describe(
        `${MessagesSchema.description} These will be prepended to every prompt sent to this LLM. Useful for setting context or influencing behavior.`
      ),
    suffix_messages: MessagesSchema.optional()
      .nullable()
      .describe(
        `${MessagesSchema.description} These will be appended to every prompt sent to this LLM. Useful for setting context or influencing behavior.`
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
      .record(z.string(), z.int().min(-100).max(100))
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

export const EnsembleLlmSchema = EnsembleLlmBaseSchema.extend({
  id: z.string().describe("The unique identifier for the Ensemble LLM."),
}).describe(
  "An LLM to be used within an Ensemble or standalone with Chat Completions, including its unique identifier."
);
export type EnsembleLlm = z.infer<typeof EnsembleLlmSchema>;

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

// Ensemble

export const EnsembleBaseSchema = z
  .object({
    llms: z
      .array(EnsembleLlmBaseWithFallbacksAndCountSchema)
      .describe("The list of LLMs that make up the ensemble."),
  })
  .describe("An ensemble of LLMs.");
export type EnsembleBase = z.infer<typeof EnsembleBaseSchema>;

export const EnsembleSchema = z
  .object({
    id: z.string().describe("The unique identifier for the Ensemble."),
    llms: z
      .array(EnsembleLlmWithFallbacksAndCountSchema)
      .describe(EnsembleBaseSchema.shape.llms.description!),
  })
  .describe("An ensemble of LLMs with a unique identifier.");
export type Ensemble = z.infer<typeof EnsembleSchema>;

export namespace Ensemble {
  export const ListItemSchema = z.object({
    id: z.string().describe("The unique identifier for the Ensemble."),
  });
  export type ListItem = z.infer<typeof ListItemSchema>;

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/ensembles", options);
    return response as { data: ListItem[] };
  }

  export const RetrieveItemSchema = EnsembleSchema.extend({
    created: z
      .uint32()
      .describe(
        "The Unix timestamp (in seconds) when the Ensemble was created."
      ),
  });
  export type RetrieveItem = z.infer<typeof RetrieveItemSchema>;

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
      .uint32()
      .describe("The total number of requests made to this Ensemble."),
    completion_tokens: z
      .uint32()
      .describe(
        "The total number of completion tokens generated by this Ensemble."
      ),
    prompt_tokens: z
      .uint32()
      .describe("The total number of prompt tokens sent to this Ensemble."),
    total_cost: z
      .number()
      .describe("The total cost incurred by using this Ensemble."),
  });
  export type HistoricalUsage = z.infer<typeof HistoricalUsageSchema>;

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
      export namespace Provider {
        export const DataCollectionSchema = z
          .enum(["allow", "deny"])
          .describe("Specifies whether to allow providers which collect data.");
        export type DataCollection = z.infer<typeof DataCollectionSchema>;

        export const SortSchema = z
          .enum(["price", "throughput", "latency"])
          .describe("Specifies the sorting strategy for provider selection.");
        export type Sort = z.infer<typeof SortSchema>;

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
      }

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

      export const ModelSchema = z
        .union([z.string(), EnsembleLlmBaseSchema])
        .describe(
          "The Ensemble LLM to use for this completion. May be a unique ID or an inline definition."
        );
      export type Model = z.infer<typeof ModelSchema>;

      export namespace ResponseFormat {
        export const TextSchema = z
          .object({
            type: z.literal("text"),
          })
          .describe("The response will be arbitrary text.")
          .meta({ title: "ResponseFormatText" });
        export type Text = z.infer<typeof TextSchema>;

        export const JsonObjectSchema = z
          .object({
            type: z.literal("json_object"),
          })
          .describe("The response will be a JSON object.")
          .meta({ title: "ResponseFormatJsonObject" });
        export type JsonObject = z.infer<typeof JsonObjectSchema>;

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
            .describe("A JSON schema definition for constraining model output.")
            .meta({ title: "ResponseFormatJsonSchemaJsonSchema" });
          export type JsonSchema = z.infer<typeof JsonSchemaSchema>;
        }

        export const JsonSchemaSchema = z
          .object({
            type: z.literal("json_schema"),
            json_schema: JsonSchema.JsonSchemaSchema,
          })
          .describe("The response will conform to the provided JSON schema.")
          .meta({ title: "ResponseFormatJsonSchema" });
        export type JsonSchema = z.infer<typeof JsonSchemaSchema>;

        export const GrammarSchema = z
          .object({
            type: z.literal("grammar"),
            grammar: z
              .string()
              .describe("The grammar definition to constrain the response."),
          })
          .describe(
            "The response will conform to the provided grammar definition."
          )
          .meta({ title: "ResponseFormatGrammar" });
        export type Grammar = z.infer<typeof GrammarSchema>;

        export const PythonSchema = z
          .object({
            type: z.literal("python"),
          })
          .describe("The response will be Python code.")
          .meta({ title: "ResponseFormatPython" });
        export type Python = z.infer<typeof PythonSchema>;
      }

      export const ResponseFormatSchema = z
        .union([
          ResponseFormat.TextSchema,
          ResponseFormat.JsonObjectSchema,
          ResponseFormat.JsonSchemaSchema,
          ResponseFormat.GrammarSchema,
          ResponseFormat.PythonSchema,
        ])
        .describe("The desired format of the model's response.")
        .meta({ title: "ResponseFormat" });
      export type ResponseFormat = z.infer<typeof ResponseFormatSchema>;

      export namespace ToolChoice {
        export namespace Function {
          export const FunctionSchema = z
            .object({
              name: z
                .string()
                .describe("The name of the function the assistant will call."),
            })
            .meta({ title: "ToolChoiceFunctionFunction" });
          export type Function = z.infer<typeof FunctionSchema>;
        }

        export const FunctionSchema = z
          .object({
            type: z.literal("function"),
            function: Function.FunctionSchema,
          })
          .describe("Specify a function for the assistant to call.")
          .meta({ title: "ToolChoiceFunction" });
        export type Function = z.infer<typeof FunctionSchema>;
      }

      export const ToolChoiceSchema = z
        .union([
          z.literal("none"),
          z.literal("auto"),
          z.literal("required"),
          ToolChoice.FunctionSchema,
        ])
        .describe("Specifies tool call behavior for the assistant.")
        .meta({ title: "ToolChoice" });
      export type ToolChoice = z.infer<typeof ToolChoiceSchema>;

      export namespace Prediction {
        export namespace Content {
          export const PartSchema = z
            .object({
              type: z.literal("text"),
              text: z.string(),
            })
            .describe("A part of the predicted content.")
            .meta({ title: "PredictionContentPart" });
          export type Part = z.infer<typeof PartSchema>;
        }

        export const ContentSchema = z.union([
          z.string().meta({ title: "PredictionContentText" }),
          z.array(Content.PartSchema).meta({ title: "PredictionContentParts" }),
        ]);
        export type Content = z.infer<typeof ContentSchema>;
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

      export const SeedSchema = z
        .bigint()
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
          messages: MessagesSchema,
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
          tools: ToolsSchema,
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

      export const StreamTrueSchema = z
        .literal(true)
        .describe("Whether to stream the response as a series of chunks.");

      export const ChatCompletionCreateParamsStreamingSchema =
        ChatCompletionCreateParamsBaseSchema.extend({
          stream: StreamTrueSchema,
        })
          .describe("Parameters for creating a streaming chat completion.")
          .meta({ title: "ChatCompletionCreateParamsStreaming" });
      export type ChatCompletionCreateParamsStreaming = z.infer<
        typeof ChatCompletionCreateParamsStreamingSchema
      >;

      export const StreamFalseSchema = z
        .literal(false)
        .describe("Whether to stream the response as a series of chunks.");

      export const ChatCompletionCreateParamsNonStreamingSchema =
        ChatCompletionCreateParamsBaseSchema.extend({
          stream: StreamFalseSchema.optional().nullable(),
        })
          .describe("Parameters for creating a unary chat completion.")
          .meta({ title: "ChatCompletionCreateParamsNonStreaming" });
      export type ChatCompletionCreateParamsNonStreaming = z.infer<
        typeof ChatCompletionCreateParamsNonStreamingSchema
      >;

      export const ChatCompletionCreateParamsSchema = z
        .union([
          ChatCompletionCreateParamsStreamingSchema,
          ChatCompletionCreateParamsNonStreamingSchema,
        ])
        .describe("Parameters for creating a chat completion.")
        .meta({ title: "ChatCompletionCreateParams" });
      export type ChatCompletionCreateParams = z.infer<
        typeof ChatCompletionCreateParamsSchema
      >;
    }

    export namespace Response {
      export const FinishReasonSchema = z
        .enum(["stop", "length", "tool_calls", "content_filter", "error"])
        .describe(
          "The reason why the assistant ceased to generate further tokens."
        );
      export type FinishReason = z.infer<typeof FinishReasonSchema>;

      export namespace Usage {
        export const CompletionTokensDetailsSchema = z
          .object({
            accepted_prediction_tokens: z
              .uint32()
              .optional()
              .describe(
                "The number of accepted prediction tokens in the completion."
              ),
            audio_tokens: z
              .uint32()
              .optional()
              .describe(
                "The number of generated audio tokens in the completion."
              ),
            reasoning_tokens: z
              .uint32()
              .optional()
              .describe(
                "The number of generated reasoning tokens in the completion."
              ),
            rejected_prediction_tokens: z
              .uint32()
              .optional()
              .describe(
                "The number of rejected prediction tokens in the completion."
              ),
          })
          .describe("Detailed breakdown of generated completion tokens.");
        export type CompletionTokensDetails = z.infer<
          typeof CompletionTokensDetailsSchema
        >;

        export const PromptTokensDetailsSchema = z
          .object({
            audio_tokens: z
              .uint32()
              .optional()
              .describe("The number of audio tokens in the prompt."),
            cached_tokens: z
              .uint32()
              .optional()
              .describe("The number of cached tokens in the prompt."),
            cache_write_tokens: z
              .uint32()
              .optional()
              .describe("The number of prompt tokens written to cache."),
            video_tokens: z
              .uint32()
              .optional()
              .describe("The number of video tokens in the prompt."),
          })
          .describe("Detailed breakdown of prompt tokens.");
        export type PromptTokensDetails = z.infer<
          typeof PromptTokensDetailsSchema
        >;

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
      }

      export const UsageSchema = z
        .object({
          completion_tokens: z
            .uint32()
            .describe("The number of tokens generated in the completion."),
          prompt_tokens: z
            .uint32()
            .describe("The number of tokens in the prompt."),
          total_tokens: z
            .uint32()
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
                .array(z.uint32())
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
        }

        export const LogprobSchema = z
          .object({
            token: z
              .string()
              .describe("The token string which was selected by the sampler."),
            bytes: z
              .array(z.uint32())
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

      export const RoleSchema = z
        .enum(["assistant"])
        .describe("The role of the message author.");
      export type Role = z.infer<typeof RoleSchema>;

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
      }

      export const ImageSchema = z
        .union([Image.ImageUrlSchema])
        .describe("An image generated by the model.");
      export type Image = z.infer<typeof ImageSchema>;

      export namespace Streaming {
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

            export const DefinitionSchema = z.object({
              name: z.string().optional().describe("The name of the function."),
              arguments: z
                .string()
                .optional()
                .describe("The arguments passed to the function."),
            });
            export type Definition = z.infer<typeof DefinitionSchema>;
          }

          export const FunctionSchema = z
            .object({
              index: z
                .uint32()
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
        }

        export const ToolCallSchema = z
          .union([ToolCall.FunctionSchema])
          .describe("A tool call made by the assistant.");
        export type ToolCall = z.infer<typeof ToolCallSchema>;

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
              .uint32()
              .describe("The index of the choice in the list of choices."),
            logprobs: LogprobsSchema.optional(),
          })
          .describe("A choice in a streaming chat completion response.");
        export type Choice = z.infer<typeof ChoiceSchema>;

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
              .uint32()
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
        export namespace ToolCall {
          export namespace Function {
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
        }

        export const ToolCallSchema = z
          .union([ToolCall.FunctionSchema])
          .describe(Streaming.ToolCallSchema.description!);
        export type ToolCall = z.infer<typeof ToolCallSchema>;

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

        export const ChoiceSchema = z
          .object({
            message: MessageSchema,
            finish_reason: FinishReasonSchema,
            index: z
              .uint32()
              .describe(Streaming.ChoiceSchema.shape.index.description!),
            logprobs: LogprobsSchema.nullable(),
          })
          .describe("A choice in a unary chat completion response.");
        export type Choice = z.infer<typeof ChoiceSchema>;

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
              .uint32()
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

      export const ProfileSchema = z
        .array(z.number())
        .describe(
          'The profile to use for the completion. Must be of the same length as the Ensemble\'s "LLMs" field, ignoring count.'
        );
      export type Profile = z.infer<typeof ProfileSchema>;

      export const VectorCompletionCreateParamsBaseSchema = z
        .object({
          retry: z
            .string()
            .optional()
            .nullable()
            .describe(
              "The unique ID of a previous incomplete or failed completion."
            ),
          messages: MessagesSchema,
          provider:
            Chat.Completions.Request.ProviderSchema.optional().nullable(),
          ensemble: EnsembleSchema,
          profile: ProfileSchema,
          seed: Chat.Completions.Request.SeedSchema.optional().nullable(),
          tools: ToolsSchema.optional()
            .nullable()
            .describe(
              `${ToolsSchema.description} These are readonly and will only be useful for explaining prior tool calls or otherwise influencing behavior.`
            ),
          responses: VectorResponsesSchema,
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

      export const VectorCompletionCreateParamsStreamingSchema =
        VectorCompletionCreateParamsBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe("Parameters for creating a streaming vector completion.")
          .meta({ title: "VectorCompletionCreateParamsStreaming" });
      export type VectorCompletionCreateParamsStreaming = z.infer<
        typeof VectorCompletionCreateParamsStreamingSchema
      >;

      export const VectorCompletionCreateParamsNonStreamingSchema =
        VectorCompletionCreateParamsBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe("Parameters for creating a unary vector completion.")
          .meta({ title: "VectorCompletionCreateParamsNonStreaming" });
      export type VectorCompletionCreateParamsNonStreaming = z.infer<
        typeof VectorCompletionCreateParamsNonStreamingSchema
      >;

      export const VectorCompletionCreateParamsSchema = z
        .union([
          VectorCompletionCreateParamsStreamingSchema,
          VectorCompletionCreateParamsNonStreamingSchema,
        ])
        .describe("Parameters for creating a vector completion.")
        .meta({ title: "VectorCompletionCreateParams" });
      export type VectorCompletionCreateParams = z.infer<
        typeof VectorCompletionCreateParamsSchema
      >;
    }

    export namespace Response {
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

      export const VoteSchema = z
        .object({
          model: z
            .string()
            .describe(
              "The unique identifier of the Ensemble LLM which generated this vote."
            ),
          ensemble_index: z
            .uint32()
            .describe("The index of the Ensemble LLM in the Ensemble."),
          flat_ensemble_index: z
            .uint32()
            .describe(
              "The flat index of the Ensemble LLM in the expanded Ensemble, accounting for counts."
            ),
          vote: z
            .array(z.number())
            .describe(
              "The vote generated by this Ensemble LLM. It is of the same length of the number of responses provided in the request. If the Ensemble LLM used logprobs, may be a probability distribution; otherwise, one of the responses will have a value of 1 and the rest 0."
            ),
          weight: z.number().describe("The weight assigned to this vote."),
          retry: z
            .boolean()
            .optional()
            .describe(
              "Whether this vote came from a previous Vector Completion which was retried."
            ),
        })
        .describe("A vote from an Ensemble LLM within a Vector Completion.");
      export type Vote = z.infer<typeof VoteSchema>;

      export const VotesSchema = z
        .array(VoteSchema)
        .describe(
          "The list of votes for responses in the request from the Ensemble LLMs within the provided Ensemble."
        );
      export type Votes = z.infer<typeof VotesSchema>;

      export namespace Scores {
        export function merged(a: number[], b: number[]): [number[], boolean] {
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

      export const ScoresSchema = z
        .array(z.number())
        .describe(
          "The scores for each response in the request, aggregated from the votes of the Ensemble LLMs."
        );
      export type Scores = z.infer<typeof ScoresSchema>;

      export namespace Weights {
        export function merged(a: number[], b: number[]): [number[], boolean] {
          return Scores.merged(a, b);
        }
      }

      export const WeightsSchema = z
        .array(z.number())
        .describe(
          "The weights assigned to each response in the request, aggregated from the votes of the Ensemble LLMs."
        );
      export type Weights = z.infer<typeof WeightsSchema>;

      export const EnsembleSchema = z
        .string()
        .describe(
          "The unique identifier of the Ensemble used for this vector completion."
        );
      export type Ensemble = z.infer<typeof EnsembleSchema>;

      export const UsageSchema = z
        .object({
          completion_tokens: z
            .uint32()
            .describe("The number of tokens generated in the completion."),
          prompt_tokens: z
            .uint32()
            .describe("The number of tokens in the prompt."),
          total_tokens: z
            .uint32()
            .describe(
              "The total number of tokens used in the prompt or generated in the completion."
            ),
          completion_tokens_details:
            Chat.Completions.Response.Usage.CompletionTokensDetailsSchema.optional(),
          prompt_tokens_details:
            Chat.Completions.Response.Usage.PromptTokensDetailsSchema.optional(),
          cost: z
            .number()
            .describe("The cost in credits incurred for this completion."),
          cost_details:
            Chat.Completions.Response.Usage.CostDetailsSchema.optional(),
          total_cost: z
            .number()
            .describe(
              "The total cost in credits incurred including upstream costs."
            ),
        })
        .describe("Token and cost usage statistics for the completion.");
      export type Usage = z.infer<typeof UsageSchema>;

      export namespace Streaming {
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

        export const ChatCompletionChunkSchema =
          Chat.Completions.Response.Streaming.ChatCompletionChunkSchema.extend({
            index: z
              .uint32()
              .describe(
                "The index of the completion amongst all chat completions."
              ),
            error: ObjectiveAIErrorSchema.optional().describe(
              "An error encountered during the generation of this chat completion."
            ),
          }).describe(
            "A chat completion chunk generated in the pursuit of a vector completion."
          );
        export type ChatCompletionChunk = z.infer<
          typeof ChatCompletionChunkSchema
        >;

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

        export const VectorCompletionChunkSchema = z
          .object({
            id: z
              .string()
              .describe("The unique identifier of the vector completion."),
            completions: z
              .array(ChatCompletionChunkSchema)
              .describe(
                "The list of chat completion chunks created for this vector completion."
              ),
            votes: VotesSchema,
            scores: ScoresSchema,
            weights: WeightsSchema,
            created: z
              .uint32()
              .describe(
                "The Unix timestamp (in seconds) when the vector completion was created."
              ),
            ensemble: EnsembleSchema,
            object: z.literal("vector.completion.chunk"),
            usage: UsageSchema.optional(),
          })
          .describe("A chunk in a streaming vector completion response.");
        export type VectorCompletionChunk = z.infer<
          typeof VectorCompletionChunkSchema
        >;
      }

      export namespace Unary {
        export const ChatCompletionSchema =
          Chat.Completions.Response.Unary.ChatCompletionSchema.extend({
            index: z
              .uint32()
              .describe(
                "The index of the completion amongst all chat completions."
              ),
            error: ObjectiveAIErrorSchema.optional().describe(
              "An error encountered during the generation of this chat completion."
            ),
          }).describe(
            "A chat completion generated in the pursuit of a vector completion."
          );
        export type ChatCompletion = z.infer<typeof ChatCompletionSchema>;

        export const VectorCompletionSchema = z
          .object({
            id: z
              .string()
              .describe("The unique identifier of the vector completion."),
            completions: z
              .array(ChatCompletionSchema)
              .describe(
                "The list of chat completions created for this vector completion."
              ),
            votes: VotesSchema,
            scores: ScoresSchema,
            weights: WeightsSchema,
            created: z
              .uint32()
              .describe(
                "The Unix timestamp (in seconds) when the vector completion was created."
              ),
            ensemble: EnsembleSchema,
            object: z.literal("vector.completion"),
            usage: UsageSchema,
          })
          .describe("A unary vector completion response.");
        export type VectorCompletion = z.infer<typeof VectorCompletionSchema>;
      }
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

// Function

export namespace Function {
  export const VectorCompletionTaskProfileSchema = z
    .object({
      ensemble: Vector.Completions.Request.EnsembleSchema,
      profile: Vector.Completions.Request.ProfileSchema,
    })
    .describe(
      "A vector completion profile for a vector completion task containing an Ensemble and array of weights."
    );
  export type VectorCompletionTaskProfile = z.infer<
    typeof VectorCompletionTaskProfileSchema
  >;

  export const RemoteFunctionTaskProfileSchema = z
    .object({
      owner: z
        .string()
        .describe("The owner of the GitHub repository containing the profile."),
      repository: z
        .string()
        .describe("The name of the GitHub repository containing the profile."),
      commit: z
        .string()
        .describe(
          "The commit SHA of the GitHub repository containing the profile."
        ),
    })
    .describe(
      "The identifiers for a function profile hosted in a GitHub repository."
    );
  export type RemoteFunctionTaskProfile = z.infer<
    typeof RemoteFunctionTaskProfileSchema
  >;

  export type InlineFunctionTaskProfile = TaskProfile[];
  export const InlineFunctionTaskProfileSchema: z.ZodType<InlineFunctionTaskProfile> =
    z
      .lazy(() =>
        z.array(TaskProfileSchema).meta({
          title: "TaskProfileArray",
          recursive: true,
        })
      )
      .describe("A function profile for a function task defined inline.");

  export const TaskProfileSchema = z
    .union([
      VectorCompletionTaskProfileSchema,
      RemoteFunctionTaskProfileSchema,
      InlineFunctionTaskProfileSchema,
    ])
    .describe("A profile for a function task.");
  export type TaskProfile = z.infer<typeof TaskProfileSchema>;

  export const InlineProfileSchema = z
    .object({
      tasks: z
        .array(TaskProfileSchema)
        .describe("The list of task profiles defined inline."),
    })
    .describe("A function profile defined inline.");
  export type InlineProfile = z.infer<typeof InlineProfileSchema>;

  export const RemoteProfileSchema = z
    .object({
      description: z.string().describe("The description of the profile."),
      changelog: z
        .string()
        .optional()
        .nullable()
        .describe(
          "When present, describes changes from the previous version or versions."
        ),
      tasks: z.array(TaskProfileSchema).describe("The list of task profiles."),
    })
    .describe("A function profile fetched from GitHub.");
  export type RemoteProfile = z.infer<typeof RemoteProfileSchema>;

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
  export const InputSchemaSchema: z.ZodType<InputSchema> = z.lazy(() =>
    z
      .union([
        InputSchema.ObjectSchema,
        InputSchema.ArraySchema,
        InputSchema.StringSchema,
        InputSchema.NumberSchema,
        InputSchema.IntegerSchema,
        InputSchema.BooleanSchema,
        InputSchema.ImageSchema,
        InputSchema.AudioSchema,
        InputSchema.VideoSchema,
        InputSchema.FileSchema,
      ])
      .describe("An input schema defining the structure of function inputs.")
  );

  export namespace InputSchema {
    export interface Object {
      type: "object";
      description?: string | null;
      properties: Record<string, InputSchema>;
      required?: string[] | null;
    }
    export const ObjectSchema: z.ZodType<Object> = z
      .object({
        type: z.literal("object"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the object input."),
        properties: z
          .record(
            z.string(),
            InputSchemaSchema.meta({
              title: "InputSchema",
              recursive: true,
            })
          )
          .describe("The properties of the object input."),
        required: z
          .array(z.string())
          .optional()
          .nullable()
          .describe("The required properties of the object input."),
      })
      .describe("An object input schema.");

    export interface Array {
      type: "array";
      description?: string | null;
      minItems?: number | null;
      maxItems?: number | null;
      items: InputSchema;
    }
    export const ArraySchema: z.ZodType<Array> = z
      .object({
        type: z.literal("array"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the array input."),
        minItems: z
          .uint32()
          .optional()
          .nullable()
          .describe("The minimum number of items in the array input."),
        maxItems: z
          .uint32()
          .optional()
          .nullable()
          .describe("The maximum number of items in the array input."),
        items: InputSchemaSchema.describe(
          "The schema of the items in the array input."
        ).meta({
          title: "InputSchema",
          recursive: true,
        }),
      })
      .describe("An array input schema.");

    export interface String {
      type: "string";
      description?: string | null;
      enum?: string[] | null;
    }
    export const StringSchema: z.ZodType<String> = z
      .object({
        type: z.literal("string"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the string input."),
        enum: z
          .array(z.string())
          .optional()
          .nullable()
          .describe("The enumeration of allowed string values."),
      })
      .describe("A string input schema.");

    export interface Number {
      type: "number";
      description?: string | null;
      minimum?: number | null;
      maximum?: number | null;
    }
    export const NumberSchema: z.ZodType<Number> = z
      .object({
        type: z.literal("number"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the number input."),
        minimum: z
          .number()
          .optional()
          .nullable()
          .describe("The minimum allowed value for the number input."),
        maximum: z
          .number()
          .optional()
          .nullable()
          .describe("The maximum allowed value for the number input."),
      })
      .describe("A number input schema.");

    export interface Integer {
      type: "integer";
      description?: string | null;
      minimum?: number | null;
      maximum?: number | null;
    }
    export const IntegerSchema: z.ZodType<Integer> = z
      .object({
        type: z.literal("integer"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the integer input."),
        minimum: z
          .uint32()
          .optional()
          .nullable()
          .describe("The minimum allowed value for the integer input."),
        maximum: z
          .uint32()
          .optional()
          .nullable()
          .describe("The maximum allowed value for the integer input."),
      })
      .describe("An integer input schema.");

    export interface Boolean {
      type: "boolean";
      description?: string | null;
    }
    export const BooleanSchema: z.ZodType<Boolean> = z
      .object({
        type: z.literal("boolean"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the boolean input."),
      })
      .describe("A boolean input schema.");

    export interface Image {
      type: "image";
      description?: string | null;
    }
    export const ImageSchema: z.ZodType<Image> = z
      .object({
        type: z.literal("image"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the image input."),
      })
      .describe("An image input schema.");

    export interface Audio {
      type: "audio";
      description?: string | null;
    }
    export const AudioSchema: z.ZodType<Audio> = z
      .object({
        type: z.literal("audio"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the audio input."),
      })
      .describe("An audio input schema.");

    export interface Video {
      type: "video";
      description?: string | null;
    }
    export const VideoSchema: z.ZodType<Video> = z
      .object({
        type: z.literal("video"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the video input."),
      })
      .describe("A video input schema.");

    export interface File {
      type: "file";
      description?: string | null;
    }
    export const FileSchema: z.ZodType<File> = z
      .object({
        type: z.literal("file"),
        description: z
          .string()
          .optional()
          .nullable()
          .describe("The description of the file input."),
      })
      .describe("A file input schema.");
  }

  export type Input =
    | Message.RichContent.Part
    | { [key: string]: Input }
    | Input[]
    | string
    | number
    | boolean;
  export const InputSchema_: z.ZodType<Input> = z
    .lazy(() =>
      z.union([
        Message.RichContent.PartSchema,
        z.record(
          z.string(),
          InputSchema_.meta({
            title: "Input",
            recursive: true,
          })
        ),
        z.array(
          InputSchema_.meta({
            title: "Input",
            recursive: true,
          })
        ),
        z.string(),
        z.number(),
        z.boolean(),
      ])
    )
    .describe("The input provided to the function.");

  export type InputExpression =
    | Message.RichContent.Part
    | { [key: string]: Expression | InputExpression }
    | (Expression | InputExpression)[]
    | string
    | number
    | boolean;
  export const InputExpressionSchema: z.ZodType<InputExpression> = z.lazy(() =>
    z
      .union([
        Message.RichContent.PartSchema,
        z.record(
          z.string(),
          InputExpressionSchema.meta({
            title: "InputExpression",
            recursive: true,
          })
        ),
        z.array(
          InputExpressionSchema.meta({
            title: "InputExpression",
            recursive: true,
          })
        ),
        z.string(),
        z.number(),
        z.boolean(),
        ExpressionSchema.describe("An expression which evaluates to an input."),
      ])
      .describe(InputSchema_.description!)
  );

  export const InputMapsExpressionSchema = z
    .union([
      ExpressionSchema.describe(
        "An expression which evaluates to a 2D array of Inputs."
      ),
      z
        .array(
          ExpressionSchema.describe(
            "An expression which evaluates to a 1D array of Inputs."
          )
        )
        .describe(
          "A list of expressions which each evaluate to a 1D array of Inputs."
        ),
    ])
    .describe(
      "An expression or list of expressions which evaluate to a 2D array of Inputs. Each sub-array will be fed into Tasks which specify an index of this input map."
    );
  export type InputMapsExpression = z.infer<typeof InputMapsExpressionSchema>;

  export namespace TaskExpression {
    export const SkipSchema = ExpressionSchema.describe(
      "An expression which evaluates to a boolean indicating whether to skip this task."
    );
    export type Skip = z.infer<typeof SkipSchema>;

    export const MapSchema = z
      .uint32()
      .describe(
        "If present, indicates that this task should be ran once for each entry in the specified input map (input map is a 2D array indexed by this value)."
      );
    export type Map = z.infer<typeof MapSchema>;

    export const ScalarFunctionSchema = z
      .object({
        type: z.literal("scalar.function"),
        owner: z
          .string()
          .describe(
            "The owner of the GitHub repository containing the function."
          ),
        repository: z
          .string()
          .describe(
            "The name of the GitHub repository containing the function."
          ),
        commit: z
          .string()
          .describe(
            "The commit SHA of the GitHub repository containing the function."
          ),
        skip: SkipSchema.optional().nullable(),
        map: MapSchema.optional().nullable(),
        input: InputExpressionSchema,
      })
      .describe("A remote published scalar function task.");
    export type ScalarFunction = z.infer<typeof ScalarFunctionSchema>;

    export const VectorFunctionSchema = z
      .object({
        type: z.literal("vector.function"),
        owner: z
          .string()
          .describe(
            "The owner of the GitHub repository containing the function."
          ),
        repository: z
          .string()
          .describe(
            "The name of the GitHub repository containing the function."
          ),
        commit: z
          .string()
          .describe(
            "The commit SHA of the GitHub repository containing the function."
          ),
        skip: SkipSchema.optional().nullable(),
        map: MapSchema.optional().nullable(),
        input: InputExpressionSchema,
      })
      .describe("A remote published vector function task.");
    export type VectorFunction = z.infer<typeof VectorFunctionSchema>;

    export const VectorCompletionSchema = z
      .object({
        type: z.literal("vector.completion"),
        skip: SkipSchema.optional().nullable(),
        map: MapSchema.optional().nullable(),
        messages: MessagesExpressionSchema,
        tools: ToolsExpressionSchema.optional()
          .nullable()
          .describe(
            `${ToolsExpressionSchema.description} These are readonly and will only be useful for explaining prior tool calls or otherwise influencing behavior.`
          ),
        responses: VectorResponsesExpressionSchema,
      })
      .describe("A vector completion task.");
    export type VectorCompletion = z.infer<typeof VectorCompletionSchema>;
  }

  export const TaskExpressionSchema = z
    .discriminatedUnion("type", [
      TaskExpression.ScalarFunctionSchema,
      TaskExpression.VectorFunctionSchema,
      TaskExpression.VectorCompletionSchema,
    ])
    .describe(
      "A task to be executed as part of the function. Will first be compiled using the parent function's input. May be skipped or mapped."
    );
  export type TaskExpression = z.infer<typeof TaskExpressionSchema>;

  export const TaskExpressionsSchema = z
    .array(TaskExpressionSchema)
    .describe("The list of tasks to be executed as part of the function.");
  export type TaskExpressions = z.infer<typeof TaskExpressionsSchema>;

  export namespace Executions {
    export namespace Request {
      // Remote Function Remote Profile

      export const FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema =
        z
          .object({
            retry_token: z
              .string()
              .optional()
              .nullable()
              .describe(
                "The retry token provided by a previous incomplete or failed function execution."
              ),
            reasoning: z
              .object({
                model: Chat.Completions.Request.ModelSchema,
                models: z
                  .array(Chat.Completions.Request.ModelSchema)
                  .optional()
                  .nullable()
                  .describe(
                    "Fallback Ensemble LLMs to use if the primary Ensemble LLM fails."
                  ),
              })
              .optional()
              .nullable()
              .describe(
                "If provided, a reasoning summary for the Function Execution will be generated. This reasoning summary attempts to detail why the final Output is what it is, based on AI assertions made during execution."
              ),
            input: InputSchema_,
            provider:
              Chat.Completions.Request.ProviderSchema.optional().nullable(),
            seed: Chat.Completions.Request.SeedSchema.optional().nullable(),
            backoff_max_elapsed_time:
              Chat.Completions.Request.BackoffMaxElapsedTimeSchema.optional().nullable(),
            first_chunk_timeout:
              Chat.Completions.Request.FirstChunkTimeoutSchema.optional().nullable(),
            other_chunk_timeout:
              Chat.Completions.Request.OtherChunkTimeoutSchema.optional().nullable(),
          })
          .describe(
            "Base parameters for executing a remote function with a remote profile."
          );
      export type FunctionExecutionParamsRemoteFunctionRemoteProfileBase =
        z.infer<
          typeof FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema
        >;

      export const FunctionExecutionParamsRemoteFunctionRemoteProfileStreamingSchema =
        FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe(
            "Parameters for executing a remote function with a remote profile and streaming the response."
          )
          .meta({
            title:
              "FunctionExecutionParamsRemoteFunctionRemoteProfileStreaming",
          });
      export type FunctionExecutionParamsRemoteFunctionRemoteProfileStreaming =
        z.infer<
          typeof FunctionExecutionParamsRemoteFunctionRemoteProfileStreamingSchema
        >;

      export const FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreamingSchema =
        FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe(
            "Parameters for executing a remote function with a remote profile with a unary response."
          )
          .meta({
            title:
              "FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreaming",
          });
      export type FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreaming =
        z.infer<
          typeof FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreamingSchema
        >;

      export const FunctionExecutionParamsRemoteFunctionRemoteProfileSchema = z
        .union([
          FunctionExecutionParamsRemoteFunctionRemoteProfileStreamingSchema,
          FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreamingSchema,
        ])
        .describe(
          "Parameters for executing a remote function with a remote profile."
        )
        .meta({ title: "FunctionExecutionParamsRemoteFunctionRemoteProfile" });
      export type FunctionExecutionParamsRemoteFunctionRemoteProfile = z.infer<
        typeof FunctionExecutionParamsRemoteFunctionRemoteProfileSchema
      >;

      // Remote Function Inline Profile

      export const FunctionExecutionParamsRemoteFunctionInlineProfileBaseSchema =
        FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema.extend({
          profile: InlineProfileSchema,
        }).describe(
          "Base parameters for executing a remote function with an inline profile."
        );
      export type FunctionExecutionParamsRemoteFunctionInlineProfileBase =
        z.infer<
          typeof FunctionExecutionParamsRemoteFunctionInlineProfileBaseSchema
        >;

      export const FunctionExecutionParamsRemoteFunctionInlineProfileStreamingSchema =
        FunctionExecutionParamsRemoteFunctionInlineProfileBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe(
            "Parameters for executing a remote function with an inline profile and streaming the response."
          )
          .meta({
            title:
              "FunctionExecutionParamsRemoteFunctionInlineProfileStreaming",
          });
      export type FunctionExecutionParamsRemoteFunctionInlineProfileStreaming =
        z.infer<
          typeof FunctionExecutionParamsRemoteFunctionInlineProfileStreamingSchema
        >;

      export const FunctionExecutionParamsRemoteFunctionInlineProfileNonStreamingSchema =
        FunctionExecutionParamsRemoteFunctionInlineProfileBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe(
            "Parameters for executing a remote function with an inline profile with a unary response."
          )
          .meta({
            title:
              "FunctionExecutionParamsRemoteFunctionInlineProfileNonStreaming",
          });
      export type FunctionExecutionParamsRemoteFunctionInlineProfileNonStreaming =
        z.infer<
          typeof FunctionExecutionParamsRemoteFunctionInlineProfileNonStreamingSchema
        >;

      export const FunctionExecutionParamsRemoteFunctionInlineProfileSchema = z
        .union([
          FunctionExecutionParamsRemoteFunctionInlineProfileStreamingSchema,
          FunctionExecutionParamsRemoteFunctionInlineProfileNonStreamingSchema,
        ])
        .describe(
          "Parameters for executing a remote function with an inline profile."
        )
        .meta({ title: "FunctionExecutionParamsRemoteFunctionInlineProfile" });
      export type FunctionExecutionParamsRemoteFunctionInlineProfile = z.infer<
        typeof FunctionExecutionParamsRemoteFunctionInlineProfileSchema
      >;

      // Inline Function Remote Profile

      export const FunctionExecutionParamsInlineFunctionRemoteProfileBaseSchema =
        FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema.extend({
          function: z.lazy(() => InlineFunctionSchema),
        }).describe(
          "Base parameters for executing an inline function with a remote profile."
        );
      export type FunctionExecutionParamsInlineFunctionRemoteProfileBase =
        z.infer<
          typeof FunctionExecutionParamsInlineFunctionRemoteProfileBaseSchema
        >;

      export const FunctionExecutionParamsInlineFunctionRemoteProfileStreamingSchema =
        FunctionExecutionParamsInlineFunctionRemoteProfileBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe(
            "Parameters for executing an inline function with a remote profile and streaming the response."
          )
          .meta({
            title:
              "FunctionExecutionParamsInlineFunctionRemoteProfileStreaming",
          });
      export type FunctionExecutionParamsInlineFunctionRemoteProfileStreaming =
        z.infer<
          typeof FunctionExecutionParamsInlineFunctionRemoteProfileStreamingSchema
        >;

      export const FunctionExecutionParamsInlineFunctionRemoteProfileNonStreamingSchema =
        FunctionExecutionParamsInlineFunctionRemoteProfileBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe(
            "Parameters for executing an inline function with a remote profile with a unary response."
          )
          .meta({
            title:
              "FunctionExecutionParamsInlineFunctionRemoteProfileNonStreaming",
          });
      export type FunctionExecutionParamsInlineFunctionRemoteProfileNonStreaming =
        z.infer<
          typeof FunctionExecutionParamsInlineFunctionRemoteProfileNonStreamingSchema
        >;

      export const FunctionExecutionParamsInlineFunctionRemoteProfileSchema = z
        .union([
          FunctionExecutionParamsInlineFunctionRemoteProfileStreamingSchema,
          FunctionExecutionParamsInlineFunctionRemoteProfileNonStreamingSchema,
        ])
        .describe(
          "Parameters for executing an inline function with a remote profile."
        )
        .meta({ title: "FunctionExecutionParamsInlineFunctionRemoteProfile" });
      export type FunctionExecutionParamsInlineFunctionRemoteProfile = z.infer<
        typeof FunctionExecutionParamsInlineFunctionRemoteProfileSchema
      >;

      // Inline Function Inline Profile

      export const FunctionExecutionParamsInlineFunctionInlineProfileBaseSchema =
        FunctionExecutionParamsRemoteFunctionRemoteProfileBaseSchema.extend({
          function: z.lazy(() => InlineFunctionSchema),
          profile: InlineProfileSchema,
        }).describe(
          "Base parameters for executing an inline function with an inline profile."
        );
      export type FunctionExecutionParamsInlineFunctionInlineProfileBase =
        z.infer<
          typeof FunctionExecutionParamsInlineFunctionInlineProfileBaseSchema
        >;

      export const FunctionExecutionParamsInlineFunctionInlineProfileStreamingSchema =
        FunctionExecutionParamsInlineFunctionInlineProfileBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe(
            "Parameters for executing an inline function with an inline profile and streaming the response."
          )
          .meta({
            title:
              "FunctionExecutionParamsInlineFunctionInlineProfileStreaming",
          });
      export type FunctionExecutionParamsInlineFunctionInlineProfileStreaming =
        z.infer<
          typeof FunctionExecutionParamsInlineFunctionInlineProfileStreamingSchema
        >;

      export const FunctionExecutionParamsInlineFunctionInlineProfileNonStreamingSchema =
        FunctionExecutionParamsInlineFunctionInlineProfileBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe(
            "Parameters for executing an inline function with an inline profile with a unary response."
          )
          .meta({
            title:
              "FunctionExecutionParamsInlineFunctionInlineProfileNonStreaming",
          });
      export type FunctionExecutionParamsInlineFunctionInlineProfileNonStreaming =
        z.infer<
          typeof FunctionExecutionParamsInlineFunctionInlineProfileNonStreamingSchema
        >;

      export const FunctionExecutionParamsInlineFunctionInlineProfileSchema = z
        .union([
          FunctionExecutionParamsInlineFunctionInlineProfileStreamingSchema,
          FunctionExecutionParamsInlineFunctionInlineProfileNonStreamingSchema,
        ])
        .describe(
          "Parameters for executing an inline function with an inline profile."
        )
        .meta({ title: "FunctionExecutionParamsInlineFunctionInlineProfile" });
      export type FunctionExecutionParamsInlineFunctionInlineProfile = z.infer<
        typeof FunctionExecutionParamsInlineFunctionInlineProfileSchema
      >;
    }

    export namespace Response {
      export namespace Task {
        export const IndexSchema = z
          .uint32()
          .describe("The index of the task in the sequence of tasks.");
        export type Index = z.infer<typeof IndexSchema>;

        export const TaskIndexSchema = z
          .uint32()
          .describe(
            "The index of the task amongst all mapped and non-skipped compiled tasks. Used internally."
          );
        export type TaskIndex = z.infer<typeof TaskIndexSchema>;

        export const TaskPathSchema = z
          .array(z.uint32())
          .describe(
            "The path of this task which may be used to navigate which nested task this is amongst the root functions tasks and sub-tasks."
          );
        export type TaskPath = z.infer<typeof TaskPathSchema>;
      }

      export namespace Streaming {
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

          export interface Function extends FunctionExecutionChunk {
            index: number;
            task_index: number;
            task_path: number[];
          }
          export const FunctionSchema: z.ZodType<Function> = z
            .lazy(() =>
              FunctionExecutionChunkSchema.extend({
                index: Task.IndexSchema,
                task_index: Task.TaskIndexSchema,
                task_path: Task.TaskPathSchema,
                tasks: z
                  .array(TaskChunkSchema)
                  .meta({
                    title: "TaskChunkArray",
                    recursive: true,
                  })
                  .describe(
                    "The tasks executed as part of the function execution."
                  ),
              })
            )
            .describe("A chunk of a function execution task.");

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

          export const VectorCompletionSchema =
            Vector.Completions.Response.Streaming.VectorCompletionChunkSchema.extend(
              {
                index: Task.IndexSchema,
                task_index: Task.TaskIndexSchema,
                task_path: Task.TaskPathSchema,
                error: ObjectiveAIErrorSchema.optional().describe(
                  "When present, indicates that an error occurred during the vector completion task."
                ),
              }
            ).describe("A chunk of a vector completion task.");
          export type VectorCompletion = z.infer<typeof VectorCompletionSchema>;
        }

        export type TaskChunk = TaskChunk.Function | TaskChunk.VectorCompletion;
        export const TaskChunkSchema: z.ZodType<TaskChunk> = z
          .union([TaskChunk.FunctionSchema, TaskChunk.VectorCompletionSchema])
          .describe("A chunk of a task execution.");

        export namespace ReasoningSummaryChunk {
          export function merged(
            a: ReasoningSummaryChunk,
            b: ReasoningSummaryChunk
          ): [ReasoningSummaryChunk, boolean] {
            const [base, baseChanged] =
              Chat.Completions.Response.Streaming.ChatCompletionChunk.merged(
                a,
                b
              );
            const [error, errorChanged] = merge(a.error, b.error);
            if (baseChanged || errorChanged) {
              return [
                {
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

        export const ReasoningSummaryChunkSchema =
          Chat.Completions.Response.Streaming.ChatCompletionChunkSchema.extend({
            error: ObjectiveAIErrorSchema.optional().describe(
              "When present, indicates that an error occurred during the chat completion."
            ),
          }).describe("A chunk of a reasoning summary generation.");
        export type ReasoningSummaryChunk = z.infer<
          typeof ReasoningSummaryChunkSchema
        >;

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
            const [reasoning, reasoningChanged] = merge(
              a.reasoning,
              b.reasoning,
              ReasoningSummaryChunk.merged
            );
            const [output, outputChanged] = merge(a.output, b.output);
            const [error, errorChanged] = merge(a.error, b.error);
            const [retry_token, retry_tokenChanged] = merge(
              a.retry_token,
              b.retry_token
            );
            const created = a.created;
            const function_ = a.function;
            const profile = a.profile;
            const object = a.object;
            const [usage, usageChanged] = merge(a.usage, b.usage);
            if (
              tasksChanged ||
              tasks_errorsChanged ||
              reasoningChanged ||
              outputChanged ||
              errorChanged ||
              retry_tokenChanged ||
              usageChanged
            ) {
              return [
                {
                  id,
                  tasks,
                  ...(tasks_errors !== undefined ? { tasks_errors } : {}),
                  ...(reasoning !== undefined ? { reasoning } : {}),
                  ...(output !== undefined ? { output } : {}),
                  ...(error !== undefined ? { error } : {}),
                  ...(retry_token !== undefined ? { retry_token } : {}),
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

        export const FunctionExecutionChunkSchema = z
          .object({
            id: z
              .string()
              .describe("The unique identifier of the function execution."),
            tasks: z
              .array(TaskChunkSchema)
              .describe(
                "The tasks executed as part of the function execution."
              ),
            tasks_errors: z
              .boolean()
              .optional()
              .describe(
                "When true, indicates that one or more tasks encountered errors during execution."
              ),
            reasoning: ReasoningSummaryChunkSchema.optional(),
            output: z
              .union([
                z
                  .number()
                  .describe("The scalar output of the function execution."),
                z
                  .array(z.number())
                  .describe("The vector output of the function execution."),
                JsonValueSchema.describe(
                  "The erroneous output of the function execution."
                ),
              ])
              .optional()
              .describe("The output of the function execution."),
            error: ObjectiveAIErrorSchema.optional().describe(
              "When present, indicates that an error occurred during the function execution."
            ),
            retry_token: z
              .string()
              .optional()
              .describe(
                "A token which may be used to retry the function execution."
              ),
            created: z
              .uint32()
              .describe(
                "The UNIX timestamp (in seconds) when the function execution chunk was created."
              ),
            function: z
              .string()
              .nullable()
              .describe(
                "The unique identifier of the function being executed."
              ),
            profile: z
              .string()
              .nullable()
              .describe("The unique identifier of the profile being used."),
            object: z
              .enum([
                "scalar.function.execution.chunk",
                "vector.function.execution.chunk",
              ])
              .describe("The object type."),
            usage: Vector.Completions.Response.UsageSchema.optional(),
          })
          .describe("A chunk of a function execution.");
        export type FunctionExecutionChunk = z.infer<
          typeof FunctionExecutionChunkSchema
        >;
      }

      export namespace Unary {
        export namespace Task {
          export interface Function extends FunctionExecution {
            index: number;
            task_index: number;
            task_path: number[];
          }
          export const FunctionSchema: z.ZodType<Function> = z
            .lazy(() =>
              FunctionExecutionSchema.extend({
                index: Response.Task.IndexSchema,
                task_index: Response.Task.TaskIndexSchema,
                task_path: Response.Task.TaskPathSchema,
                tasks: z
                  .array(TaskSchema)
                  .meta({
                    title: "TaskArray",
                    recursive: true,
                  })
                  .describe(
                    "The tasks executed as part of the function execution."
                  ),
              })
            )
            .describe("A function execution task.");

          export const VectorCompletionSchema =
            Vector.Completions.Response.Unary.VectorCompletionSchema.extend({
              index: Response.Task.IndexSchema,
              task_index: Response.Task.TaskIndexSchema,
              task_path: Response.Task.TaskPathSchema,
              error: ObjectiveAIErrorSchema.nullable().describe(
                "When non-null, indicates that an error occurred during the vector completion task."
              ),
            }).describe("A vector completion task.");
          export type VectorCompletion = z.infer<typeof VectorCompletionSchema>;
        }

        export type Task = Task.Function | Task.VectorCompletion;
        export const TaskSchema: z.ZodType<Task> = z
          .union([Task.FunctionSchema, Task.VectorCompletionSchema])
          .describe("A task execution.");

        export const ReasoningSummarySchema =
          Chat.Completions.Response.Unary.ChatCompletionSchema.extend({
            error: ObjectiveAIErrorSchema.nullable().describe(
              "When non-null, indicates that an error occurred during the chat completion."
            ),
          }).describe("A reasoning summary generation.");
        export type ReasoningSummary = z.infer<typeof ReasoningSummarySchema>;

        export const FunctionExecutionSchema = z
          .object({
            id: z
              .string()
              .describe("The unique identifier of the function execution."),
            tasks: z
              .array(TaskSchema)
              .describe(
                "The tasks executed as part of the function execution."
              ),
            tasks_errors: z
              .boolean()
              .describe(
                "When true, indicates that one or more tasks encountered errors during execution."
              ),
            reasoning: ReasoningSummarySchema.nullable(),
            output: z
              .union([
                z
                  .number()
                  .describe("The scalar output of the function execution."),
                z
                  .array(z.number())
                  .describe("The vector output of the function execution."),
                JsonValueSchema.describe(
                  "The erroneous output of the function execution."
                ),
              ])
              .describe("The output of the function execution."),
            error: ObjectiveAIErrorSchema.nullable().describe(
              "When non-null, indicates that an error occurred during the function execution."
            ),
            retry_token: z
              .string()
              .nullable()
              .describe(
                "A token which may be used to retry the function execution."
              ),
            created: z
              .uint32()
              .describe(
                "The UNIX timestamp (in seconds) when the function execution chunk was created."
              ),
            function: z
              .string()
              .nullable()
              .describe(
                "The unique identifier of the function being executed."
              ),
            profile: z
              .string()
              .nullable()
              .describe("The unique identifier of the profile being used."),
            object: z
              .enum(["scalar.function.execution", "vector.function.execution"])
              .describe("The object type."),
            usage: Vector.Completions.Response.UsageSchema,
          })
          .describe("A function execution.");
        export type FunctionExecution = z.infer<typeof FunctionExecutionSchema>;
      }
    }
  }

  export namespace ComputeProfile {
    export namespace Request {
      export namespace DatasetItem {
        export namespace Target {
          export const ScalarSchema = z
            .object({
              type: z.literal("scalar"),
              value: z.number(),
            })
            .describe(
              "A scalar target output. The desired output is this exact scalar."
            );
          export type Scalar = z.infer<typeof ScalarSchema>;

          export const VectorSchema = z
            .object({
              type: z.literal("vector"),
              value: z.array(z.number()),
            })
            .describe(
              "A vector target output. The desired output is this exact vector."
            );
          export type Vector = z.infer<typeof VectorSchema>;

          export const VectorWinnerSchema = z
            .object({
              type: z.literal("vector_winner"),
              value: z.uint32(),
            })
            .describe(
              "A vector winner target output. The desired output is a vector where the highest value is at the specified index."
            );
          export type VectorWinner = z.infer<typeof VectorWinnerSchema>;
        }

        export const TargetSchema = z
          .discriminatedUnion("type", [
            Target.ScalarSchema,
            Target.VectorSchema,
            Target.VectorWinnerSchema,
          ])
          .describe("The target output for a given function input.");
        export type Target = z.infer<typeof TargetSchema>;
      }

      export const DatasetItemSchema = z
        .object({
          input: InputSchema_,
          target: DatasetItem.TargetSchema,
        })
        .describe("A Function input and its corresponding target output.");
      export type DatasetItem = z.infer<typeof DatasetItemSchema>;

      // Remote Function

      export const FunctionComputeProfileParamsRemoteFunctionBaseSchema = z
        .object({
          retry_token: z
            .string()
            .optional()
            .nullable()
            .describe(
              "The retry token provided by a previous incomplete or failed profile computation."
            ),
          max_retries: z
            .uint32()
            .optional()
            .nullable()
            .describe(
              "The maximum number of retries to attempt when a function execution fails during profile computation."
            ),
          n: z
            .uint32()
            .describe(
              "The number of function executions to perform per dataset item. Generally speaking, higher N values increase the quality of the computed profile."
            ),
          dataset: z
            .array(DatasetItemSchema)
            .describe(
              "The dataset of input and target output pairs to use for computing the profile."
            ),
          ensemble: Vector.Completions.Request.EnsembleSchema,
          provider:
            Chat.Completions.Request.ProviderSchema.optional().nullable(),
          seed: Chat.Completions.Request.SeedSchema.optional().nullable(),
          backoff_max_elapsed_time:
            Chat.Completions.Request.BackoffMaxElapsedTimeSchema.optional().nullable(),
          first_chunk_timeout:
            Chat.Completions.Request.FirstChunkTimeoutSchema.optional().nullable(),
          other_chunk_timeout:
            Chat.Completions.Request.OtherChunkTimeoutSchema.optional().nullable(),
        })
        .describe(
          "Base parameters for computing a function profile for a remote function."
        );
      export type FunctionComputeProfileParamsRemoteFunctionBase = z.infer<
        typeof FunctionComputeProfileParamsRemoteFunctionBaseSchema
      >;

      export const FunctionComputeProfileParamsRemoteFunctionStreamingSchema =
        FunctionComputeProfileParamsRemoteFunctionBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe(
            "Parameters for computing a function profile for a remote function and streaming the response."
          )
          .meta({ title: "FunctionComputeProfileParamsStreaming" });
      export type FunctionComputeProfileParamsRemoteFunctionStreaming = z.infer<
        typeof FunctionComputeProfileParamsRemoteFunctionStreamingSchema
      >;

      export const FunctionComputeProfileParamsRemoteFunctionNonStreamingSchema =
        FunctionComputeProfileParamsRemoteFunctionBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe(
            "Parameters for computing a function profile for a remote function with a unary response."
          )
          .meta({ title: "FunctionComputeProfileParamsNonStreaming" });
      export type FunctionComputeProfileParamsRemoteFunctionNonStreaming =
        z.infer<
          typeof FunctionComputeProfileParamsRemoteFunctionNonStreamingSchema
        >;

      export const FunctionComputeProfileParamsRemoteFunctionSchema = z
        .union([
          FunctionComputeProfileParamsRemoteFunctionStreamingSchema,
          FunctionComputeProfileParamsRemoteFunctionNonStreamingSchema,
        ])
        .describe(
          "Parameters for computing a function profile for a remote function."
        )
        .meta({ title: "FunctionComputeProfileParams" });
      export type FunctionComputeProfileParamsRemoteFunction = z.infer<
        typeof FunctionComputeProfileParamsRemoteFunctionSchema
      >;

      // Inline Function

      export const FunctionComputeProfileParamsInlineFunctionBaseSchema =
        FunctionComputeProfileParamsRemoteFunctionBaseSchema.extend({
          function: z.lazy(() => InlineFunctionSchema),
        }).describe(
          "Base parameters for computing a function profile for an inline function."
        );
      export type FunctionComputeProfileParamsInlineFunctionBase = z.infer<
        typeof FunctionComputeProfileParamsInlineFunctionBaseSchema
      >;

      export const FunctionComputeProfileParamsInlineFunctionStreamingSchema =
        FunctionComputeProfileParamsInlineFunctionBaseSchema.extend({
          stream: Chat.Completions.Request.StreamTrueSchema,
        })
          .describe(
            "Parameters for computing a function profile for an inline function and streaming the response."
          )
          .meta({
            title: "FunctionComputeProfileParamsInlineFunctionStreaming",
          });
      export type FunctionComputeProfileParamsInlineFunctionStreaming = z.infer<
        typeof FunctionComputeProfileParamsInlineFunctionStreamingSchema
      >;

      export const FunctionComputeProfileParamsInlineFunctionNonStreamingSchema =
        FunctionComputeProfileParamsInlineFunctionBaseSchema.extend({
          stream:
            Chat.Completions.Request.StreamFalseSchema.optional().nullable(),
        })
          .describe(
            "Parameters for computing a function profile for an inline function with a unary response."
          )
          .meta({
            title: "FunctionComputeProfileParamsInlineFunctionNonStreaming",
          });
      export type FunctionComputeProfileParamsInlineFunctionNonStreaming =
        z.infer<
          typeof FunctionComputeProfileParamsInlineFunctionNonStreamingSchema
        >;

      export const FunctionComputeProfileParamsInlineFunctionSchema = z
        .union([
          FunctionComputeProfileParamsInlineFunctionStreamingSchema,
          FunctionComputeProfileParamsInlineFunctionNonStreamingSchema,
        ])
        .describe(
          "Parameters for computing a function profile for an inline function."
        )
        .meta({ title: "FunctionComputeProfileParamsInlineFunction" });
      export type FunctionComputeProfileParamsInlineFunction = z.infer<
        typeof FunctionComputeProfileParamsInlineFunctionSchema
      >;
    }

    export namespace Response {
      export const FittingStatsSchema = z
        .object({
          loss: z
            .number()
            .describe("The final sum loss achieved during weights fitting."),
          executions: z
            .uint32()
            .describe(
              "The total number of function executions used during weights fitting."
            ),
          starts: z
            .uint32()
            .describe(
              "The number of fitting starts attempted. Each start begins with a randomized weight vector."
            ),
          rounds: z
            .uint32()
            .describe(
              "The number of fitting rounds performed across all starts."
            ),
          errors: z
            .uint32()
            .describe(
              "The number of errors which occured while computing outputs during fitting."
            ),
        })
        .describe(
          "Statistics about the fitting process used to compute the weights for the profile."
        );
      export type FittingStats = z.infer<typeof FittingStatsSchema>;

      export namespace Streaming {
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

        export const FunctionExecutionChunkSchema =
          Executions.Response.Streaming.FunctionExecutionChunkSchema.extend({
            index: z
              .uint32()
              .describe(
                "The index of the function execution chunk in the list of executions."
              ),
            dataset: z
              .uint32()
              .describe(
                "The index of the dataset item this function execution chunk corresponds to."
              ),
            n: z
              .uint32()
              .describe(
                "The N index for this function execution chunk. There will be N function executions, and N comes from the request parameters."
              ),
            retry: z
              .uint32()
              .describe(
                "The retry index for this function execution chunk. There may be multiple retries for a given dataset item and N index."
              ),
          }).describe(
            "A chunk of a function execution ran during profile computation."
          );
        export type FunctionExecutionChunk = z.infer<
          typeof FunctionExecutionChunkSchema
        >;

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

        export const FunctionComputeProfileChunkSchema = z
          .object({
            id: z
              .string()
              .describe(
                "The unique identifier of the function profile computation chunk."
              ),
            executions: z
              .array(FunctionExecutionChunkSchema)
              .describe(
                "The function executions performed as part of computing the profile."
              ),
            executions_errors: z
              .boolean()
              .optional()
              .describe(
                "When true, indicates that one or more function executions encountered errors during profile computation."
              ),
            profile: InlineProfileSchema.optional(),
            fitting_stats: FittingStatsSchema.optional(),
            created: z
              .uint32()
              .describe(
                "The UNIX timestamp (in seconds) when the function profile computation was created."
              ),
            function: z
              .string()
              .describe(
                "The unique identifier of the function for which the profile is being computed."
              ),
            object: z.literal("function.compute.profile.chunk"),
            usage: Vector.Completions.Response.UsageSchema.optional(),
          })
          .describe("A chunk of a function profile computation.");
        export type FunctionComputeProfileChunk = z.infer<
          typeof FunctionComputeProfileChunkSchema
        >;
      }

      export namespace Unary {
        export const FunctionExecutionSchema =
          Executions.Response.Unary.FunctionExecutionSchema.extend({
            index: z
              .uint32()
              .describe(
                "The index of the function execution in the list of executions."
              ),
            dataset: z
              .uint32()
              .describe(
                "The index of the dataset item this function execution corresponds to."
              ),
            n: z
              .uint32()
              .describe(
                "The N index for this function execution. There will be N function executions, and N comes from the request parameters."
              ),
            retry: z
              .uint32()
              .describe(
                "The retry index for this function execution. There may be multiple retries for a given dataset item and N index."
              ),
          }).describe("A function execution ran during profile computation.");
        export type FunctionExecution = z.infer<typeof FunctionExecutionSchema>;

        export const FunctionComputeProfileSchema = z
          .object({
            id: z
              .string()
              .describe(
                "The unique identifier of the function profile computation."
              ),
            executions: z
              .array(FunctionExecutionSchema)
              .describe(
                "The function executions performed as part of computing the profile."
              ),
            executions_errors: z
              .boolean()
              .describe(
                "When true, indicates that one or more function executions encountered errors during profile computation."
              ),
            profile: InlineProfileSchema,
            fitting_stats: FittingStatsSchema,
            created: z
              .uint32()
              .describe(
                "The UNIX timestamp (in seconds) when the function profile computation was created."
              ),
            function: z
              .string()
              .describe(
                "The unique identifier of the function for which the profile is being computed."
              ),
            object: z.literal("function.compute.profile"),
            usage: Vector.Completions.Response.UsageSchema,
          })
          .describe("A function profile computation.");
        export type FunctionComputeProfile = z.infer<
          typeof FunctionComputeProfileSchema
        >;
      }
    }
  }

  export namespace Profile {
    export const ListItemSchema = z.object({
      owner: z
        .string()
        .describe("The owner of the GitHub repository containing the profile."),
      repository: z
        .string()
        .describe("The name of the GitHub repository containing the profile."),
      commit: z
        .string()
        .describe(
          "The commit SHA of the GitHub repository containing the profile."
        ),
    });
    export type ListItem = z.infer<typeof ListItemSchema>;

    export async function list(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<ListItem[]> {
      const response = await openai.get("/functions/profiles", options);
      return response as ListItem[];
    }

    export const HistoricalUsageSchema = z.object({
      requests: z
        .uint32()
        .describe(
          "The total number of requests made to Functions while using this Profile."
        ),
      completion_tokens: z
        .uint32()
        .describe(
          "The total number of completion tokens generated by Functions while using this Profile."
        ),
      prompt_tokens: z
        .uint32()
        .describe(
          "The total number of prompt tokens sent to Functions while using this Profile."
        ),
      total_cost: z
        .number()
        .describe("The total cost incurred by using this Profile."),
    });
    export type HistoricalUsage = z.infer<typeof HistoricalUsageSchema>;

    export async function retrieveUsage(
      openai: OpenAI,
      powner: string,
      prepository: string,
      pcommit: string | null | undefined,
      options?: OpenAI.RequestOptions
    ): Promise<HistoricalUsage> {
      const response = await openai.get(
        pcommit !== null && pcommit !== undefined
          ? `/functions/profiles/${powner}/${prepository}/${pcommit}/usage`
          : `/functions/profiles/${powner}/${prepository}/usage`,
        options
      );
      return response as HistoricalUsage;
    }
  }

  export async function executeInlineFunctionInlineProfile(
    openai: OpenAI,
    body: Executions.Request.FunctionExecutionParamsInlineFunctionInlineProfileStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function executeInlineFunctionInlineProfile(
    openai: OpenAI,
    body: Executions.Request.FunctionExecutionParamsInlineFunctionInlineProfileNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function executeInlineFunctionInlineProfile(
    openai: OpenAI,
    body: Executions.Request.FunctionExecutionParamsInlineFunctionInlineProfile,
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

  export async function executeRemoteFunctionInlineProfile(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionInlineProfileStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function executeRemoteFunctionInlineProfile(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionInlineProfileNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function executeRemoteFunctionInlineProfile(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionInlineProfile,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    const response = await openai.post(
      fcommit !== null && fcommit !== undefined
        ? `/functions/${fowner}/${frepository}/${fcommit}`
        : `/functions/${fowner}/${frepository}`,
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

  export async function executeInlineFunctionRemoteProfile(
    openai: OpenAI,
    powner: string,
    prepository: string,
    pcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsInlineFunctionRemoteProfileStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function executeInlineFunctionRemoteProfile(
    openai: OpenAI,
    powner: string,
    prepository: string,
    pcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsInlineFunctionRemoteProfileNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function executeInlineFunctionRemoteProfile(
    openai: OpenAI,
    powner: string,
    prepository: string,
    pcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsInlineFunctionRemoteProfile,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    const response = await openai.post(
      pcommit !== null && pcommit !== undefined
        ? `/functions/profiles/${powner}/${prepository}/${pcommit}`
        : `/functions/profiles/${powner}/${prepository}`,
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

  export async function executeRemoteFunctionRemoteProfile(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    powner: string,
    prepository: string,
    pcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfileStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function executeRemoteFunctionRemoteProfile(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    powner: string,
    prepository: string,
    pcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function executeRemoteFunctionRemoteProfile(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    powner: string,
    prepository: string,
    pcommit: string | null | undefined,
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfile,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    let url: string;
    if (fcommit !== null && fcommit !== undefined) {
      if (pcommit !== null && pcommit !== undefined) {
        url = `/functions/${fowner}/${frepository}/${fcommit}/profiles/${powner}/${prepository}/${pcommit}`;
      } else {
        url = `/functions/${fowner}/${frepository}/${fcommit}/profiles/${powner}/${prepository}`;
      }
    } else if (pcommit !== null && pcommit !== undefined) {
      url = `/functions/${fowner}/${frepository}/profiles/${powner}/${prepository}/${pcommit}`;
    } else {
      url = `/functions/${fowner}/${frepository}/profiles/${powner}/${prepository}`;
    }
    const response = await openai.post(url, {
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
    function_:
      | InlineFunction
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    profile:
      | InlineProfile
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfileStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Stream<Executions.Response.Streaming.FunctionExecutionChunk>>;
  export async function execute(
    openai: OpenAI,
    function_:
      | InlineFunction
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    profile:
      | InlineProfile
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfileNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<Executions.Response.Unary.FunctionExecution>;
  export async function execute(
    openai: OpenAI,
    function_:
      | InlineFunction
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    profile:
      | InlineProfile
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    body: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfile,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<Executions.Response.Streaming.FunctionExecutionChunk>
    | Executions.Response.Unary.FunctionExecution
  > {
    if ("owner" in function_ && "repository" in function_) {
      if ("owner" in profile && "repository" in profile) {
        const requestBody: Executions.Request.FunctionExecutionParamsRemoteFunctionRemoteProfile =
          body;
        return executeRemoteFunctionRemoteProfile(
          openai,
          function_.owner,
          function_.repository,
          function_.commit,
          profile.owner,
          profile.repository,
          profile.commit,
          requestBody as any,
          options
        );
      } else {
        const requestBody: Executions.Request.FunctionExecutionParamsRemoteFunctionInlineProfile =
          {
            ...body,
            profile,
          };
        return executeRemoteFunctionInlineProfile(
          openai,
          function_.owner,
          function_.repository,
          function_.commit,
          requestBody as any,
          options
        );
      }
    } else if ("owner" in profile && "repository" in profile) {
      const requestBody: Executions.Request.FunctionExecutionParamsInlineFunctionRemoteProfile =
        {
          ...body,
          function: function_,
        };
      return executeInlineFunctionRemoteProfile(
        openai,
        profile.owner,
        profile.repository,
        profile.commit,
        requestBody as any,
        options
      );
    } else {
      const requestBody: Executions.Request.FunctionExecutionParamsInlineFunctionInlineProfile =
        {
          ...body,
          function: function_,
          profile,
        };
      return executeInlineFunctionInlineProfile(
        openai,
        requestBody as any,
        options
      );
    }
  }

  export async function computeProfileInlineFunction(
    openai: OpenAI,
    body: ComputeProfile.Request.FunctionComputeProfileParamsInlineFunctionStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<
    Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
  >;
  export async function computeProfileInlineFunction(
    openai: OpenAI,
    body: ComputeProfile.Request.FunctionComputeProfileParamsInlineFunctionNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<ComputeProfile.Response.Unary.FunctionComputeProfile>;
  export async function computeProfileInlineFunction(
    openai: OpenAI,
    body: ComputeProfile.Request.FunctionComputeProfileParamsInlineFunction,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
    | ComputeProfile.Response.Unary.FunctionComputeProfile
  > {
    const response = await openai.post("/functions/profiles/compute", {
      body,
      stream: body.stream ?? false,
      ...options,
    });
    return response as
      | Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
      | ComputeProfile.Response.Unary.FunctionComputeProfile;
  }

  export async function computeProfileRemoteFunction(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    body: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunctionStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<
    Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
  >;
  export async function computeProfileRemoteFunction(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    body: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunctionNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<ComputeProfile.Response.Unary.FunctionComputeProfile>;
  export async function computeProfileRemoteFunction(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    body: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunction,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
    | ComputeProfile.Response.Unary.FunctionComputeProfile
  > {
    const response = await openai.post(
      fcommit !== null && fcommit !== undefined
        ? `/functions/${fowner}/${frepository}/${fcommit}/profiles/compute`
        : `/functions/${fowner}/${frepository}/profiles/compute`,
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

  export async function computeProfile(
    openai: OpenAI,
    function_:
      | InlineFunction
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    body: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunctionStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<
    Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
  >;
  export async function computeProfile(
    openai: OpenAI,
    function_:
      | InlineFunction
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    body: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunctionNonStreaming,
    options?: OpenAI.RequestOptions
  ): Promise<ComputeProfile.Response.Unary.FunctionComputeProfile>;
  export async function computeProfile(
    openai: OpenAI,
    function_:
      | InlineFunction
      | {
          owner: string;
          repository: string;
          commit?: string | null | undefined;
        },
    body: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunction,
    options?: OpenAI.RequestOptions
  ): Promise<
    | Stream<ComputeProfile.Response.Streaming.FunctionComputeProfileChunk>
    | ComputeProfile.Response.Unary.FunctionComputeProfile
  > {
    if ("owner" in function_ && "repository" in function_) {
      const requestBody: ComputeProfile.Request.FunctionComputeProfileParamsRemoteFunction =
        body;
      return computeProfileRemoteFunction(
        openai,
        function_.owner,
        function_.repository,
        function_.commit,
        requestBody as any,
        options
      );
    } else {
      const requestBody: ComputeProfile.Request.FunctionComputeProfileParamsInlineFunction =
        {
          ...body,
          function: function_,
        };
      return computeProfileInlineFunction(openai, requestBody as any, options);
    }
  }

  export const ListItemSchema = z.object({
    owner: z
      .string()
      .describe("The owner of the GitHub repository containing the function."),
    repository: z
      .string()
      .describe("The name of the GitHub repository containing the function."),
    commit: z
      .string()
      .describe(
        "The commit SHA of the GitHub repository containing the function."
      ),
  });
  export type ListItem = z.infer<typeof ListItemSchema>;

  export async function list(
    openai: OpenAI,
    options?: OpenAI.RequestOptions
  ): Promise<{ data: ListItem[] }> {
    const response = await openai.get("/functions", options);
    return response as { data: ListItem[] };
  }

  export const HistoricalUsageSchema = z.object({
    requests: z
      .uint32()
      .describe("The total number of requests made to this Function."),
    completion_tokens: z
      .uint32()
      .describe(
        "The total number of completion tokens generated by this Function."
      ),
    prompt_tokens: z
      .uint32()
      .describe("The total number of prompt tokens sent to this Function."),
    total_cost: z
      .number()
      .describe("The total cost incurred by using this Function."),
  });
  export type HistoricalUsage = z.infer<typeof HistoricalUsageSchema>;

  export async function retrieveUsage(
    openai: OpenAI,
    fowner: string,
    frepository: string,
    fcommit: string | null | undefined,
    options?: OpenAI.RequestOptions
  ): Promise<HistoricalUsage> {
    const response = await openai.get(
      fcommit !== null && fcommit !== undefined
        ? `/functions/${fowner}/${frepository}/${fcommit}/usage`
        : `/functions/${fowner}/${frepository}/usage`,
      options
    );
    return response as HistoricalUsage;
  }
}

export const RemoteScalarFunctionSchema = z
  .object({
    type: z.literal("scalar.function"),
    description: z.string().describe("The description of the scalar function."),
    changelog: z
      .string()
      .optional()
      .nullable()
      .describe(
        "When present, describes changes from the previous version or versions."
      ),
    input_schema: Function.InputSchemaSchema,
    input_maps: Function.InputMapsExpressionSchema.optional().nullable(),
    tasks: Function.TaskExpressionsSchema,
    output: ExpressionSchema.describe(
      "An expression which evaluates to a single number. This is the output of the scalar function. Will be provided with the outputs of all tasks."
    ),
  })
  .describe("A scalar function fetched from GitHub.")
  .meta({ title: "RemoteScalarFunction" });
export type RemoteScalarFunction = z.infer<typeof RemoteScalarFunctionSchema>;

export const RemoteVectorFunctionSchema = z
  .object({
    type: z.literal("vector.function"),
    description: z.string().describe("The description of the vector function."),
    changelog: z
      .string()
      .optional()
      .nullable()
      .describe(
        "When present, describes changes from the previous version or versions."
      ),
    input_schema: Function.InputSchemaSchema,
    input_maps: Function.InputMapsExpressionSchema.optional().nullable(),
    tasks: Function.TaskExpressionsSchema,
    output: ExpressionSchema.describe(
      "An expression which evaluates to an array of numbers. This is the output of the vector function. Will be provided with the outputs of all tasks."
    ),
    output_length: z
      .union([
        z.uint32().describe("The fixed length of the output vector."),
        ExpressionSchema.describe(
          "An expression which evaluates to the length of the output vector. Will only be provided with the function input. The output length must be determinable from the input alone."
        ),
      ])
      .describe("The length of the output vector."),
  })
  .describe("A vector function fetched from GitHub.")
  .meta({ title: "RemoteVectorFunction" });
export type RemoteVectorFunction = z.infer<typeof RemoteVectorFunctionSchema>;

export const RemoteFunctionSchema = z
  .discriminatedUnion("type", [
    RemoteScalarFunctionSchema,
    RemoteVectorFunctionSchema,
  ])
  .describe("A remote function fetched from GitHub.");
export type RemoteFunction = z.infer<typeof RemoteFunctionSchema>;

export const InlineScalarFunctionSchema = z
  .object({
    type: z.literal("scalar.function"),
    input_maps: Function.InputMapsExpressionSchema.optional().nullable(),
    tasks: Function.TaskExpressionsSchema,
    output: ExpressionSchema.describe(
      "An expression which evaluates to a single number. This is the output of the scalar function. Will be provided with the outputs of all tasks."
    ),
  })
  .describe("A scalar function defined inline.")
  .meta({ title: "InlineScalarFunction" });
export type InlineScalarFunction = z.infer<typeof InlineScalarFunctionSchema>;

export const InlineVectorFunctionSchema = z
  .object({
    type: z.literal("vector.function"),
    input_maps: Function.InputMapsExpressionSchema.optional().nullable(),
    tasks: Function.TaskExpressionsSchema,
    output: ExpressionSchema.describe(
      "An expression which evaluates to an array of numbers. This is the output of the vector function. Will be provided with the outputs of all tasks."
    ),
  })
  .describe("A vector function defined inline.")
  .meta({ title: "InlineVectorFunction" });
export type InlineVectorFunction = z.infer<typeof InlineVectorFunctionSchema>;

export const InlineFunctionSchema = z
  .discriminatedUnion("type", [
    InlineScalarFunctionSchema,
    InlineVectorFunctionSchema,
  ])
  .describe("A function defined inline.");
export type InlineFunction = z.infer<typeof InlineFunctionSchema>;

export namespace Auth {
  export const ApiKeySchema = z.object({
    api_key: z.string().describe("The API key."),
    created: z
      .string()
      .describe("The RFC 3339 timestamp when the API key was created."),
    expires: z
      .string()
      .nullable()
      .describe(
        "The RFC 3339 timestamp when the API key expires, or null if it does not expire."
      ),
    disabled: z
      .string()
      .nullable()
      .describe(
        "The RFC 3339 timestamp when the API key was disabled, or null if it is not disabled."
      ),
    name: z.string().describe("The name of the API key."),
    description: z
      .string()
      .nullable()
      .describe(
        "The description of the API key, or null if no description was provided."
      ),
  });
  export type ApiKey = z.infer<typeof ApiKeySchema>;

  export const ApiKeyWithCostSchema = ApiKeySchema.extend({
    cost: z
      .number()
      .describe("The total cost incurred while using this API key."),
  });
  export type ApiKeyWithCost = z.infer<typeof ApiKeyWithCostSchema>;

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

  export const OpenRouterApiKeySchema = z.object({
    api_key: z.string().describe("The OpenRouter API key."),
  });
  export type OpenRouterApiKey = z.infer<typeof OpenRouterApiKeySchema>;

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

  export const CreditsSchema = z.object({
    credits: z.number().describe("The current number of credits available."),
    total_credits_purchased: z
      .number()
      .describe("The total number of credits ever purchased."),
    total_credits_used: z
      .number()
      .describe("The total number of credits ever used."),
  });
  export type Credits = z.infer<typeof CreditsSchema>;

  export namespace Credits {
    export async function retrieve(
      openai: OpenAI,
      options?: OpenAI.RequestOptions
    ): Promise<Credits> {
      const response = await openai.get("/auth/credits", options);
      return response as Credits;
    }
  }
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
