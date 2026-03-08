import type { Metadata } from "next";
import { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Functions",
  description:
    "Browse and execute ObjectiveAI scoring functions. Score, rank, and simulate preferences using ensembles of LLMs.",
};

export default function FunctionsLayout({ children }: { children: ReactNode }) {
  return children;
}
