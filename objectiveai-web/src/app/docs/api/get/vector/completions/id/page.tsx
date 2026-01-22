import { getServerSession } from "next-auth";
import { Provider } from "@/provider";
import { ProviderServer } from "@/provider_server";
import { headers as getHeaders } from "next/headers";
import { EndpointDocs } from "@/components/docs/EndpointDocs";
import z from "zod";
import { Vector } from "objectiveai";

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
      requestPath={z.object({
        id: z
          .string()
          .describe("The unique identifier of the vector completion."),
      })}
      responseBody={Vector.Completions.RetrieveSchema}
    />
  );
}
