import type { Metadata } from "next";
import { Inter, Roboto_Mono } from "next/font/google";
import "./globals.css";
import { cookies as getCookies } from "next/headers";
import { ReactElement } from "react";
import cn from "classnames";

const inter = Inter({
  subsets: ["latin", "latin-ext"],
  variable: "--font-sans",
});
const roboto_mono = Roboto_Mono({
  subsets: ["latin", "latin-ext"],
  variable: "--font-mono",
});

export const metadata: Metadata = {
  title: "ObjectiveAI - LLM Confidence",
  description:
    "A Completions API with a Confidence Score for every choice. Powered by AI Diversity.",
  icons: {
    icon: "/favicon.svg",
  },
};

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}): Promise<ReactElement> {
  const cookies = await getCookies();
  const theme = cookies.get("theme")?.value;
  const validTheme = theme === "dark" || theme === "light";
  return (
    <html lang="en">
      <body
        className={cn(
          inter.className,
          inter.variable,
          roboto_mono.variable,
          validTheme && theme
        )}
      >
        {children}
      </body>
    </html>
  );
}
