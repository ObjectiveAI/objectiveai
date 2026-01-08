import { Mutex } from "async-mutex";
import { DefaultSession, Session } from "next-auth";
import { getSession } from "next-auth/react";

export type Provider =
  | Provider.Google
  | Provider.GitHub
  | Provider.Twitter
  | Provider.Reddit
  | Provider.IP;

export namespace Provider {
  export type Token =
    | Google.Token
    | GitHub.Token
    | Twitter.Token
    | Reddit.Token
    | IP.Token;

  export type ServerToken =
    | Google.ServerToken
    | GitHub.ServerToken
    | Twitter.ServerToken
    | Reddit.ServerToken
    | IP.ServerToken;

  export namespace Token {
    export function fromServerToken(serverToken: ServerToken): Token {
      if (serverToken.provider === "google") {
        return Google.Token.fromServerToken(serverToken);
      } else if (serverToken.provider === "github") {
        return GitHub.Token.fromServerToken(serverToken);
      } else if (serverToken.provider === "twitter") {
        return Twitter.Token.fromServerToken(serverToken);
      } else if (serverToken.provider === "reddit") {
        return Reddit.Token.fromServerToken(serverToken);
      } else {
        throw new Error("unreachable");
      }
    }
  }

  export interface TokenSession {
    mu?: Mutex;
    token?: Token;
    session?: DefaultSession;
  }

  export namespace TokenSession {
    export function fromSession({
      provider_token,
      ...session
    }: Session): TokenSession {
      return { token: provider_token, session };
    }

    export async function authorization(
      tokenSession: TokenSession
    ): Promise<string | null> {
      let authorization = authorizationInner(tokenSession);
      if (authorization) return authorization;
      if (tokenSession.mu === undefined) tokenSession.mu = new Mutex();
      const releaseMu = await tokenSession.mu.acquire();
      try {
        authorization = authorizationInner(tokenSession);
        if (authorization) return authorization;
        const session = await getSession();
        tokenSession.token = session?.provider_token;
        return authorizationInner(tokenSession);
      } finally {
        releaseMu();
      }
    }

    function authorizationInner(tokenSession: TokenSession): string | null {
      if (!tokenSession.token) {
        return null;
      } else if (tokenSession.token.provider === "google") {
        return Google.Token.authorization(tokenSession.token);
      } else if (tokenSession.token.provider === "github") {
        return GitHub.Token.authorization(tokenSession.token);
      } else if (tokenSession.token.provider === "twitter") {
        return Twitter.Token.authorization(tokenSession.token);
      } else if (tokenSession.token.provider === "reddit") {
        return Reddit.Token.authorization(tokenSession.token);
      } else if (tokenSession.token.provider === "ip") {
        return IP.Token.authorization(tokenSession.token);
      } else {
        throw new Error("unreachable");
      }
    }
  }

  // Google

  export type Google = "google";

  export namespace Google {
    export interface Token {
      provider: Google;
      id_token: string;
      expires_at: number;
    }

    export interface ServerToken extends Token {
      refresh_token: string;
    }

    export namespace Token {
      export function fromServerToken(serverToken: ServerToken): Token {
        const { provider, id_token, expires_at } = serverToken;
        return { provider, id_token, expires_at };
      }

      export function authorization(token: Token): string | null {
        if (Date.now() + 60_000 < token.expires_at) {
          return token.id_token;
        } else {
          return null;
        }
      }
    }
  }

  // GitHub

  export type GitHub = "github";

  export namespace GitHub {
    export interface Token {
      provider: GitHub;
      access_token: string;
    }

    export type ServerToken = Token;

    export namespace Token {
      export function fromServerToken(serverToken: ServerToken): Token {
        const { provider, access_token } = serverToken;
        return { provider, access_token };
      }

      export function authorization(token: Token): string {
        return token.access_token;
      }
    }
  }

  // Twitter

  export type Twitter = "twitter";

  export namespace Twitter {
    export interface Token {
      provider: Twitter;
      access_token: string;
      expires_at: number;
    }

    export interface ServerToken extends Token {
      refresh_token: string;
    }

    export namespace Token {
      export function fromServerToken(serverToken: ServerToken): Token {
        const { provider, access_token, expires_at } = serverToken;
        return { provider, access_token, expires_at };
      }

      export function authorization(token: Token): string | null {
        if (Date.now() + 60_000 < token.expires_at) {
          return token.access_token;
        } else {
          return null;
        }
      }
    }
  }

  // Reddit

  export type Reddit = "reddit";

  export namespace Reddit {
    export interface Token {
      provider: Reddit;
      access_token: string;
      expires_at: number;
    }

    export interface ServerToken extends Token {
      refresh_token: string;
    }

    export namespace Token {
      export function fromServerToken(serverToken: ServerToken): Token {
        const { provider, access_token, expires_at } = serverToken;
        return { provider, access_token, expires_at };
      }

      export function authorization(token: Token): string | null {
        if (Date.now() + 60_000 < token.expires_at) {
          return token.access_token;
        } else {
          return null;
        }
      }
    }
  }

  // IP

  export type IP = "ip";

  export namespace IP {
    export interface Token {
      provider: IP;
      authorization: string;
    }

    export type ServerToken = Token;

    export namespace Token {
      export function fromServerToken(serverToken: ServerToken): Token {
        const { provider, authorization } = serverToken;
        return { provider, authorization };
      }

      export function authorization(token: Token): string {
        return token.authorization;
      }
    }
  }
}
