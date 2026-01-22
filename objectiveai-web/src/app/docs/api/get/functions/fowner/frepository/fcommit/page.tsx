import { getServerSession } from "next-auth";
import { Provider } from "@/provider";
import { ProviderServer } from "@/provider_server";
import { headers as getHeaders } from "next/headers";
import { EndpointDocs } from "@/components/docs/EndpointDocs";
import z from "zod";
import { Functions } from "objectiveai";

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
      requestPath={z.object({
        fowner: z
          .string()
          .describe(
            "The owner of the GitHub repository containing the function."
          ),
        frepository: z
          .string()
          .describe(
            "The name of the GitHub repository containing the function."
          ),
        fcommit: z
          .string()
          .optional()
          .describe(
            "The commit SHA of the GitHub repository containing the function."
          ),
      })}
      responseBody={Functions.RetrieveSchema}
    />
  );
}
