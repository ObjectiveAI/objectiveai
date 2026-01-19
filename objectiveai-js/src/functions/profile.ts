import { EnsembleSchema } from "src/vector/completions/request/ensemble";
import { ProfileSchema as VectorProfileSchema } from "src/vector/completions/request/profile";
import z from "zod";

// Task Profile

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
  )
  .meta({ title: "RemoteFunctionTaskProfile" });
export type RemoteFunctionTaskProfile = z.infer<
  typeof RemoteFunctionTaskProfileSchema
>;

export interface InlineFunctionTaskProfile {
  tasks: TaskProfile[];
}
export const InlineFunctionTaskProfileSchema: z.ZodType<InlineFunctionTaskProfile> =
  z
    .object({
      tasks: z
        .array(
          z
            .lazy(() => TaskProfileSchema)
            .meta({ title: "TaskProfile", recursive: true })
        )
        .describe("The list of task profiles."),
    })
    .describe("A function profile defined inline.")
    .meta({ title: "InlineFunctionTaskProfile" });

export const VectorCompletionTaskProfileSchema = z
  .object({
    ensemble: EnsembleSchema,
    profile: VectorProfileSchema,
  })
  .describe(
    "A vector completion profile for a vector completion task containing an Ensemble and array of weights."
  );
export type VectorCompletionTaskProfile = z.infer<
  typeof VectorCompletionTaskProfileSchema
>;

export const TaskProfileSchema = z
  .union([
    RemoteFunctionTaskProfileSchema,
    InlineFunctionTaskProfileSchema,
    VectorCompletionTaskProfileSchema,
  ])
  .describe("The profile for a task.");
export type TaskProfile = z.infer<typeof TaskProfileSchema>;

export const TaskProfilesSchema = z
  .array(TaskProfileSchema)
  .describe("The list of task profiles.");
export type TaskProfiles = z.infer<typeof TaskProfilesSchema>;

// Inline Profile

export const InlineProfileSchema = z
  .object({
    tasks: TaskProfilesSchema,
  })
  .describe("A function profile defined inline.")
  .meta({ title: "InlineProfile" });
export type InlineProfile = z.infer<typeof InlineProfileSchema>;

// Remote Profile

export const RemoteProfileSchema = InlineProfileSchema.extend({
  description: z.string().describe("The description of the profile."),
  changelog: z
    .string()
    .optional()
    .nullable()
    .describe(
      "When present, describes changes from the previous version or versions."
    ),
})
  .describe('A function profile fetched from GitHub. "profile.json"')
  .meta({ title: "RemoteProfile" });
export type RemoteProfile = z.infer<typeof RemoteProfileSchema>;

// Profile

export const ProfileSchema = z
  .union([InlineProfileSchema, RemoteProfileSchema])
  .describe("A function profile.")
  .meta({ title: "Profile" });
export type Profile = z.infer<typeof ProfileSchema>;
