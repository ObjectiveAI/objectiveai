import type { NextConfig } from "next";
import path from "path";

const securityHeaders = [
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
];

const nextConfig: NextConfig = {
  output: "standalone",
  outputFileTracingRoot: path.join(import.meta.dirname, ".."),
  transpilePackages: ["objectiveai", "@objectiveai/function-tree"],
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "lh3.googleusercontent.com",
        pathname: "/**",
      },
      {
        protocol: "https",
        hostname: "avatars.githubusercontent.com",
        pathname: "/**",
      },
      {
        protocol: "https",
        hostname: "pbs.twimg.com",
        pathname: "/**",
      },
    ],
  },
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: securityHeaders,
      },
    ];
  },
  async redirects() {
    return [
      {
        source: '/vibe-native',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/faq',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/sdk-first',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/chat',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/vector',
        destination: '/docs',
        permanent: false,
      },
      // #84: Remove Create Functions
      {
        source: '/functions/create',
        destination: '/functions',
        permanent: false,
      },
      // #88: Remove Profiles browse
      {
        source: '/profiles',
        destination: '/functions',
        permanent: false,
      },
      // #87: Remove Train Profile
      {
        source: '/profiles/train',
        destination: '/functions',
        permanent: false,
      },
      // #91: Remove Ensembles section
      {
        source: '/ensembles',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/ensembles/create',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/ensembles/:path*',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/ensemble-llms',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/ensemble-llms/create',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/ensemble-llms/:path*',
        destination: '/docs',
        permanent: false,
      },
      {
        source: '/information',
        destination: '/docs',
        permanent: false,
      },
    ];
  },
};

export default nextConfig;
