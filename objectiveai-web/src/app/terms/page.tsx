import { getServerSession } from "next-auth";
import { Terms } from "@/components/legal/Terms";
import { Provider } from "@/provider";
import { ProviderServer } from "@/provider_server";
import { headers as getHeaders } from "next/headers";

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
    <Terms
      session={
        session
          ? Provider.TokenSession.fromSession(session)
          : ProviderServer.TokenSession.fromIpHeader(headers)
      }
    />
  );
}
