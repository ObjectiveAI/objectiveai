"use client";

import { useEffect } from "react";
import Tracker from "@sonarly/tracker";

export default function SonarlyTracker() {
  useEffect(() => {
    const projectKey = process.env.NEXT_PUBLIC_SONARLY_PROJECT_KEY;
    const ingestPoint = process.env.NEXT_PUBLIC_SONARLY_INGEST_POINT;
    if (!projectKey || !ingestPoint) return;

    new Tracker({
      projectKey,
      ingestPoint,
      __FORCE_RECORD: true,
    }).start();
  }, []);

  return null;
}
