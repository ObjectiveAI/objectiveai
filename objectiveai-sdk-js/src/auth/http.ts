import { ObjectiveAI, type RequestOptions } from "../client";
import type { AuthCreateApiKeyRequest } from "./createApiKeyRequest";
import type { AuthApiKeyWithMetadata } from "./apiKeyWithMetadata";
import type { AuthDisableApiKeyRequest } from "./disableApiKeyRequest";
import type { AuthListApiKeyResponse } from "./listApiKeyResponse";
import type { AuthGetCreditsResponse } from "./getCreditsResponse";
import type { AuthCreateOpenRouterByokApiKeyRequest } from "./createOpenRouterByokApiKeyRequest";
import type { AuthGetOpenRouterByokApiKeyResponse } from "./getOpenRouterByokApiKeyResponse";

export function authCreateApiKey(
  client: ObjectiveAI,
  body: AuthCreateApiKeyRequest,
  options?: RequestOptions,
): Promise<AuthApiKeyWithMetadata> {
  return client.post_unary<AuthApiKeyWithMetadata>("/auth/keys", body, options);
}

export function authCreateOpenrouterByokApiKey(
  client: ObjectiveAI,
  body: AuthCreateOpenRouterByokApiKeyRequest,
  options?: RequestOptions,
): Promise<AuthGetOpenRouterByokApiKeyResponse> {
  return client.post_unary<AuthGetOpenRouterByokApiKeyResponse>(
    "/auth/keys/openrouter",
    body,
    options,
  );
}

export function authDisableApiKey(
  client: ObjectiveAI,
  body: AuthDisableApiKeyRequest,
  options?: RequestOptions,
): Promise<AuthApiKeyWithMetadata> {
  return client.delete_unary<AuthApiKeyWithMetadata>("/auth/keys", body, options);
}

export function authDeleteOpenrouterByokApiKey(
  client: ObjectiveAI,
  options?: RequestOptions,
): Promise<void> {
  return client.delete_unary<void>("/auth/keys/openrouter", undefined, options);
}

export function authListApiKeys(
  client: ObjectiveAI,
  options?: RequestOptions,
): Promise<AuthListApiKeyResponse> {
  return client.get_unary<AuthListApiKeyResponse>("/auth/keys", options);
}

export function authGetOpenrouterByokApiKey(
  client: ObjectiveAI,
  options?: RequestOptions,
): Promise<AuthGetOpenRouterByokApiKeyResponse> {
  return client.get_unary<AuthGetOpenRouterByokApiKeyResponse>(
    "/auth/keys/openrouter",
    options,
  );
}

export function authGetCredits(
  client: ObjectiveAI,
  options?: RequestOptions,
): Promise<AuthGetCreditsResponse> {
  return client.get_unary<AuthGetCreditsResponse>("/auth/credits", options);
}
