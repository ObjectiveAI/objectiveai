"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function ChatPage() {
  const router = useRouter();
  useEffect(() => {
    router.replace("/coming-soon");
  }, [router]);
  return null;
}
