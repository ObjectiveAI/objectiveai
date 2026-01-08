import { getServerSession } from "next-auth";
import { Provider } from "@/provider";
import { ProviderServer } from "@/provider_server";
import { headers as getHeaders } from "next/headers";
import { EndpointDocs } from "@/components/docs/EndpointDocs";
import z from "zod";
import { Auth } from "objectiveai";

export default async function Page() {
  const [headers, session] = await Promise.all([
    getHeaders(),
    getServerSession({
      callbacks: {
        async session({ session, token: { provider_token } }) {
          return Object.assign({}, session, { provider_token });
        },
      },
    }),
  ]);
  return (
    <EndpointDocs
      session={
        session
          ? Provider.TokenSession.fromSession(session)
          : ProviderServer.TokenSession.fromIpHeader(headers)
      }
      requestHeaders={z.object({
        authorization: z.string().describe("Authorization token (required)."),
      })}
      requestBody={z.object({
        name: z.string().describe("Name for the new API key."),
        expires: z
          .string()
          .optional()
          .describe("Expiration date for the new API key in RFC 3339 format."),
        description: z
          .string()
          .optional()
          .describe("Description for the new API key."),
      })}
      responseBody={Auth.ApiKeySchema}
    />
  );
}
