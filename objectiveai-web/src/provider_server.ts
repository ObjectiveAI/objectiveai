import { ReadonlyHeaders } from "next/dist/server/web/spec-extension/adapters/headers";
import crypto from "crypto";
import { Provider } from "./provider";

export namespace ProviderServer {
  export namespace TokenSession {
    export function fromIpHeader(
      headers: ReadonlyHeaders
    ): Provider.TokenSession | undefined {
      const ips = headers.get(process.env.USER_IP_HEADER!);
      if (ips) {
        // get last entry
        const ips_split = ips.split(",");
        return IP.Token.session(ips_split[ips_split.length - 1]);
      }
    }
  }

  export namespace IP {
    export namespace Token {
      export function session(ip: string): Provider.TokenSession {
        return {
          token: {
            provider: "ip",
            authorization: crypto
              .publicEncrypt(
                {
                  key: process.env.IP_RSA_PUBLIC_KEY!,
                  padding: crypto.constants.RSA_PKCS1_PADDING,
                },
                Buffer.from(ip)
              )
              .toString("base64"),
          },
        };
      }
    }
  }
}
