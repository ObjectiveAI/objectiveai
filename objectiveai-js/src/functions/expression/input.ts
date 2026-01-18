import {
  AudioRichContentPart,
  AudioRichContentPartSchema,
  FileRichContentPart,
  FileRichContentPartSchema,
  ImageRichContentPart,
  ImageRichContentPartSchema,
  VideoRichContentPart,
  VideoRichContentPartSchema,
} from "src/chat/completions/request/message";
import z from "zod";
import { Expression, ExpressionSchema } from "./expression";

// Input Schema

export interface ObjectInputSchema {
  type: "object";
  description?: string | null;
  properties: Record<string, InputSchema>;
  required?: string[] | null;
}
export const ObjectInputSchemaSchema: z.ZodType<Object> = z
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
        z
          .lazy(() => InputSchemaSchema)
          .meta({
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
  .describe("An object input schema.")
  .meta({ title: "ObjectInputSchema" });

export interface ArrayInputSchema {
  type: "array";
  description?: string | null;
  minItems?: number | null;
  maxItems?: number | null;
  items: InputSchema;
}
export const ArrayInputSchemaSchema: z.ZodType<ArrayInputSchema> = z
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
    items: z
      .lazy(() => InputSchemaSchema)
      .meta({
        title: "InputSchema",
        recursive: true,
      }),
  })
  .describe("An array input schema.")
  .meta({ title: "ArrayInputSchema" });

export const StringInputSchemaSchema = z
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
  .describe("A string input schema.")
  .meta({ title: "StringInputSchema" });
export type StringInputSchema = z.infer<typeof StringInputSchemaSchema>;

export const NumberInputSchemaSchema = z
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
  .describe("A number input schema.")
  .meta({ title: "NumberInputSchema" });
export type NumberInputSchema = z.infer<typeof NumberInputSchemaSchema>;

export const IntegerInputSchemaSchema = z
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
  .describe("An integer input schema.")
  .meta({ title: "IntegerInputSchema" });
export type IntegerInputSchema = z.infer<typeof IntegerInputSchemaSchema>;

export const BooleanInputSchemaSchema = z
  .object({
    type: z.literal("boolean"),
    description: z
      .string()
      .optional()
      .nullable()
      .describe("The description of the boolean input."),
  })
  .describe("A boolean input schema.")
  .meta({ title: "BooleanInputSchema" });
export type BooleanInputSchema = z.infer<typeof BooleanInputSchemaSchema>;

export const ImageInputSchemaSchema = z
  .object({
    type: z.literal("image"),
    description: z
      .string()
      .optional()
      .nullable()
      .describe("The description of the image input."),
  })
  .describe("An image input schema.")
  .meta({ title: "ImageInputSchema" });
export type ImageInputSchema = z.infer<typeof ImageInputSchemaSchema>;

export const AudioInputSchemaSchema = z
  .object({
    type: z.literal("audio"),
    description: z
      .string()
      .optional()
      .nullable()
      .describe("The description of the audio input."),
  })
  .describe("An audio input schema.")
  .meta({ title: "AudioInputSchema" });
export type AudioInputSchema = z.infer<typeof AudioInputSchemaSchema>;

export const VideoInputSchemaSchema = z
  .object({
    type: z.literal("video"),
    description: z
      .string()
      .optional()
      .nullable()
      .describe("The description of the video input."),
  })
  .describe("A video input schema.")
  .meta({ title: "VideoInputSchema" });
export type VideoInputSchema = z.infer<typeof VideoInputSchemaSchema>;

export const FileInputSchemaSchema = z
  .object({
    type: z.literal("file"),
    description: z
      .string()
      .optional()
      .nullable()
      .describe("The description of the file input."),
  })
  .describe("A file input schema.")
  .meta({ title: "FileInputSchema" });
export type FileInputSchema = z.infer<typeof FileInputSchemaSchema>;

export interface AnyOfInputSchema {
  anyOf: InputSchema[];
}
export const AnyOfInputSchemaSchema: z.ZodType<AnyOfInputSchema> = z
  .object({
    anyOf: z
      .array(
        z
          .lazy(() => InputSchemaSchema)
          .meta({
            title: "InputSchema",
            recursive: true,
          })
      )
      .describe("The possible schemas that the input can match."),
  })
  .describe("A union of schemas - input must match at least one.")
  .meta({ title: "AnyOfInputSchema" });

export const InputSchemaSchema = z
  .union([
    ObjectInputSchemaSchema,
    ArrayInputSchemaSchema,
    StringInputSchemaSchema,
    NumberInputSchemaSchema,
    IntegerInputSchemaSchema,
    BooleanInputSchemaSchema,
    ImageInputSchemaSchema,
    AudioInputSchemaSchema,
    VideoInputSchemaSchema,
    FileInputSchemaSchema,
    AnyOfInputSchemaSchema,
  ])
  .describe("An input schema defining the structure of function inputs.")
  .meta({ title: "InputSchema" });
export type InputSchema = z.infer<typeof InputSchemaSchema>;

// Input Value

export type InputValue =
  | ImageRichContentPart
  | AudioRichContentPart
  | VideoRichContentPart
  | FileRichContentPart
  | { [key: string]: InputValue }
  | InputValue[]
  | string
  | number
  | boolean;
export const InputValueSchema: z.ZodType<InputValue> = z
  .union([
    ImageRichContentPartSchema,
    AudioRichContentPartSchema,
    VideoRichContentPartSchema,
    FileRichContentPartSchema,
    z.record(
      z.string(),
      z
        .lazy(() => InputValueSchema)
        .meta({
          title: "InputValue",
          recursive: true,
        })
    ),
    z.array(
      z
        .lazy(() => InputValueSchema)
        .meta({
          title: "InputValue",
          recursive: true,
        })
    ),
    z.string(),
    z.number(),
    z.boolean(),
  ])
  .describe("A value provided as input to a function.")
  .meta({ title: "InputValue" });

export type InputValueExpression =
  | ImageRichContentPart
  | AudioRichContentPart
  | VideoRichContentPart
  | FileRichContentPart
  | { [key: string]: InputValueExpression }
  | InputValueExpression[]
  | string
  | number
  | boolean
  | Expression;
export const InputValueExpressionSchema: z.ZodType<InputValueExpression> = z
  .union([
    ImageRichContentPartSchema,
    AudioRichContentPartSchema,
    VideoRichContentPartSchema,
    FileRichContentPartSchema,
    z.record(
      z.string(),
      z
        .lazy(() => InputValueExpressionSchema)
        .meta({
          title: "InputValueExpression",
          recursive: true,
        })
    ),
    z.array(
      z
        .lazy(() => InputValueExpressionSchema)
        .meta({
          title: "InputValueExpression",
          recursive: true,
        })
    ),
    z.string(),
    z.number(),
    z.boolean(),
    ExpressionSchema.describe(
      "An expression which evaluates to an input value."
    ),
  ])
  .describe(InputValueSchema.description!)
  .meta({ title: "InputValueExpression" });

// Input Maps

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
