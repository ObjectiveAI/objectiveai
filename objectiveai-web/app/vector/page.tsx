"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function VectorCompletionsPage() {
  const router = useRouter();
  useEffect(() => {
    router.replace("/coming-soon");
  }, [router]);
  return null;
}
