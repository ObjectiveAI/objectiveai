import { ObjectiveAI, type RequestOptions } from "../../client";
import type { RemotePathCommitOptional } from "../../remotePathCommitOptional";
import type { FunctionsProfilesListProfilesRequest } from "./listProfilesRequest";
import type { FunctionsProfilesListProfileResponse } from "./listProfileResponse";
import type { FunctionsProfilesGetProfileResponse } from "./getProfileResponse";
import type { FunctionsProfilesUsageProfileResponse } from "./usageProfileResponse";

export function functionsProfilesListProfiles(
  client: ObjectiveAI,
  params: FunctionsProfilesListProfilesRequest,
  options?: RequestOptions,
): Promise<FunctionsProfilesListProfileResponse> {
  return client.post_unary<FunctionsProfilesListProfileResponse>("functions/profiles/list", params, options);
}

export function functionsProfilesGetProfile(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsProfilesGetProfileResponse> {
  return client.post_unary<FunctionsProfilesGetProfileResponse>("functions/profiles", params, options);
}

export function functionsProfilesGetProfileUsage(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsProfilesUsageProfileResponse> {
  return client.post_unary<FunctionsProfilesUsageProfileResponse>("functions/profiles/usage", params, options);
}
