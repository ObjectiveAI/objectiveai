import { getServerSession } from "next-auth";
import { Credits } from "@/components/account/credits/Credits";
import { Provider } from "@/provider";

export default async function Page() {
  const session = await getServerSession({
    callbacks: {
      async session({ session, token: { provider_token } }) {
        return Object.assign({}, session, { provider_token });
      },
    },
  });
  return <Credits session={Provider.TokenSession.fromSession(session!)} />;
}
