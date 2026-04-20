import type { Metadata } from "next";
import { Geist } from "next/font/google";
import { JetBrains_Mono } from "next/font/google";
import { Shell } from "@/components/Shell";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-jetbrains-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: {
    default: "ObjectiveAI",
    template: "%s | ObjectiveAI",
  },
  description: "The agentic collective judgment harness.",
  metadataBase: new URL("https://objective-ai.io"),
  openGraph: {
    title: "ObjectiveAI",
    description: "The agentic collective judgment harness.",
    url: "https://objective-ai.io",
    siteName: "ObjectiveAI",
    locale: "en_US",
    type: "website",
  },
  twitter: {
    card: "summary",
    title: "ObjectiveAI",
    description: "The agentic collective judgment harness.",
    creator: "@objectv_ai",
  },
  robots: {
    index: true,
    follow: true,
  },
  icons: {
    icon: "/favicon.ico",
    apple: "/apple-touch-icon.png",
  },
  manifest: "/site.webmanifest",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`${geistSans.variable} ${jetbrainsMono.variable}`}>
      <body>
        <Shell>{children}</Shell>
      </body>
    </html>
  );
}
