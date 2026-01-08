import { getServerSession } from "next-auth";
import { Keys } from "@/components/account/keys/Keys";
import { Provider } from "@/provider";

export default async function Page() {
  const session = await getServerSession({
    callbacks: {
      async session({ session, token: { provider_token } }) {
        return Object.assign({}, session, { provider_token });
      },
    },
  });
  return <Keys session={Provider.TokenSession.fromSession(session!)} />;
}
