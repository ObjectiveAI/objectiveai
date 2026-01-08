import { Provider } from "@/provider";
import NextAuth from "next-auth";
import GoogleProvider from "next-auth/providers/google";
import GitHubProvider from "next-auth/providers/github";
import TwitterProvider from "next-auth/providers/twitter";
import RedditProvider from "next-auth/providers/reddit";

function getEnv(key: string): string {
  return (
    process.env[key] ??
    (() => {
      throw new Error(`${key} is not defined`);
    })()
  );
}

const authGoogleClientId = getEnv("AUTH_GOOGLE_CLIENT_ID");
const authGoogleClientSecret = getEnv("AUTH_GOOGLE_CLIENT_SECRET");
const authGitHubClientId = getEnv("AUTH_GITHUB_CLIENT_ID");
const authGitHubClientSecret = getEnv("AUTH_GITHUB_CLIENT_SECRET");
const authTwitterClientId = getEnv("AUTH_TWITTER_CLIENT_ID");
const authTwitterClientSecret = getEnv("AUTH_TWITTER_CLIENT_SECRET");
const authRedditClientId = getEnv("AUTH_REDDIT_CLIENT_ID");
const authRedditClientSecret = getEnv("AUTH_REDDIT_CLIENT_SECRET");

const handler = NextAuth({
  providers: [
    GoogleProvider({
      clientId: authGoogleClientId,
      clientSecret: authGoogleClientSecret,
      authorization: {
        params: {
          scope: "openid profile",
          access_type: "offline",
          prompt: "consent",
        },
      },
    }),
    GitHubProvider({
      clientId: authGitHubClientId,
      clientSecret: authGitHubClientSecret,
      authorization: { params: { scope: "read:user" } },
    }),
    TwitterProvider({
      clientId: authTwitterClientId,
      clientSecret: authTwitterClientSecret,
      authorization: {
        params: { scope: "users.read tweet.read offline.access" },
      },
      version: "2.0",
    }),
    RedditProvider({
      clientId: authRedditClientId,
      clientSecret: authRedditClientSecret,
      authorization: { params: { scope: "identity", duration: "permanent" } },
    }),
  ],
  callbacks: {
    async jwt({ token, account }) {
      if (account === null || account === undefined) {
        const { provider_token } = token;
        if (
          provider_token.provider === "google" &&
          Date.now() + 60_000 >= provider_token.expires_at
        ) {
          const res = await fetch("https://oauth2.googleapis.com/token", {
            method: "POST",
            headers: {
              "Content-Type": "application/x-www-form-urlencoded",
            },
            body: new URLSearchParams({
              client_id: authGoogleClientId,
              client_secret: authGoogleClientSecret,
              grant_type: "refresh_token",
              refresh_token: provider_token.refresh_token,
            }),
          });
          if (!res.ok) throw new Error(await res.text());
          const { id_token, expires_in } = await res.json();
          provider_token.id_token = id_token;
          provider_token.expires_at = Date.now() + expires_in * 1000;
        } else if (
          provider_token.provider === "twitter" &&
          Date.now() + 60_000 >= provider_token.expires_at
        ) {
          const res = await fetch("https://api.twitter.com/2/oauth2/token", {
            method: "POST",
            headers: {
              Authorization: `Basic ${Buffer.from(
                `${authTwitterClientId}:${authTwitterClientSecret}`
              ).toString("base64")}`,
              "Content-Type": "application/x-www-form-urlencoded",
            },
            body: new URLSearchParams({
              grant_type: "refresh_token",
              refresh_token: provider_token.refresh_token,
            }),
          });
          if (!res.ok) throw new Error(await res.text());
          const { refresh_token, access_token, expires_in } = await res.json();
          provider_token.refresh_token =
            refresh_token ?? provider_token.refresh_token;
          provider_token.access_token = access_token;
          provider_token.expires_at = Date.now() + expires_in * 1000;
        } else if (
          provider_token.provider === "reddit" &&
          Date.now() + 60_000 >= provider_token.expires_at
        ) {
          const res = await fetch(
            "https://www.reddit.com/api/v1/access_token",
            {
              method: "POST",
              headers: {
                Authorization: `Basic ${Buffer.from(
                  `${authRedditClientId}:${authRedditClientSecret}`
                ).toString("base64")}`,
                "Content-Type": "application/x-www-form-urlencoded",
                "User-Agent": "ObjectiveAI", // todo env
              },
              body: new URLSearchParams({
                grant_type: "refresh_token",
                refresh_token: provider_token.refresh_token,
              }),
            }
          );
          if (!res.ok) throw new Error(await res.text());
          const { access_token, expires_in } = await res.json();
          provider_token.access_token = access_token;
          provider_token.expires_at = Date.now() + expires_in * 1000;
        }
        return token;
      } else {
        const { provider, refresh_token, access_token, id_token, expires_at } =
          account;
        if (provider === "google") {
          if (
            refresh_token === undefined ||
            access_token === undefined ||
            id_token === undefined ||
            expires_at === undefined
          ) {
            throw new Error("unreachable");
          }
          const provider_token: Provider.Google.ServerToken = {
            provider: "google",
            refresh_token,
            id_token,
            expires_at,
          };
          return Object.assign({}, token, { provider_token });
        } else if (provider === "github") {
          if (access_token === undefined) {
            throw new Error("unreachable");
          }
          const provider_token: Provider.GitHub.ServerToken = {
            provider: "github",
            access_token,
          };
          return Object.assign({}, token, { provider_token });
        } else if (provider === "twitter") {
          if (
            refresh_token === undefined ||
            access_token === undefined ||
            expires_at === undefined
          ) {
            throw new Error("unreachable");
          }
          const provider_token: Provider.Twitter.ServerToken = {
            provider: "twitter",
            refresh_token,
            access_token,
            expires_at,
          };
          return Object.assign({}, token, { provider_token });
        } else if (provider === "reddit") {
          if (
            refresh_token === undefined ||
            access_token === undefined ||
            expires_at === undefined
          ) {
            throw new Error("unreachable");
          }
          const provider_token: Provider.Reddit.ServerToken = {
            provider: "reddit",
            refresh_token,
            access_token,
            expires_at,
          };
          return Object.assign({}, token, { provider_token });
        } else {
          throw new Error("unreachable");
        }
      }
    },
    async session({ session, token: { provider_token } }) {
      return Object.assign({}, session, {
        provider_token: Provider.Token.fromServerToken(provider_token),
      });
    },
  },
});

export { handler as GET, handler as POST };
