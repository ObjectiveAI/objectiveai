import type { Metadata } from "next";
import { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Legal",
};

export default function LegalLayout({ children }: { children: ReactNode }) {
  return children;
}
