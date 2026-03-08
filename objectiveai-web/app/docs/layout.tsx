import type { Metadata } from "next";
import { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Docs",
};

export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="page">
      {children}
    </div>
  );
}
