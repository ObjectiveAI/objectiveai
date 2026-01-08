import { ObjectiveAIError } from "objectiveai";

export function toObjectiveAiError(error: unknown): ObjectiveAIError {
  if (
    typeof error !== "object" ||
    error === null ||
    !("message" in error) ||
    typeof error.message !== "string"
  ) {
    return {
      message: "Unknown error",
      code: 500,
    };
  }
  const errorMessage = error.message;
  try {
    const splitIndex = errorMessage.indexOf(" ");
    if (splitIndex > 0) {
      const codeStr = errorMessage.slice(0, splitIndex);
      const code = parseInt(codeStr, 10);
      if (Number.isInteger(code) && code > 99 && code < 600) {
        return {
          code,
          message: errorMessage.slice(splitIndex + 1),
        };
      }
    }
  } catch {}
  return { code: 500, message: errorMessage };
}
