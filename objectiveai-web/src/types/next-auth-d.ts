import { DefaultSession } from "next-auth";
import { DefaultJWT } from "next-auth/jwt";
import { Provider } from "@/provider";

declare module "next-auth" {
  interface Session extends DefaultSession {
    provider_token: Provider.Token;
  }
}

declare module "next-auth/jwt" {
  interface JWT extends Record<string, unknown>, DefaultJWT {
    provider_token: Provider.ServerToken;
  }
}
