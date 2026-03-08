import type { Metadata } from "next";
import { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Team",
};

export default function PeopleLayout({ children }: { children: ReactNode }) {
  return children;
}
