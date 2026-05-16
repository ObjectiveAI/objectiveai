import { ObjectiveAI } from "@objectiveai/sdk";

/** Shared ObjectiveAI client */
let _client: ObjectiveAI | null = null;

export function getClient(): ObjectiveAI {
  if (!_client) {
    _client = new ObjectiveAI();
  }
  return _client;
}
