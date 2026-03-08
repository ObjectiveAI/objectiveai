"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function SDKFirstPage() {
  const router = useRouter();
  useEffect(() => {
    router.replace("/coming-soon");
  }, [router]);
  return null;
}
