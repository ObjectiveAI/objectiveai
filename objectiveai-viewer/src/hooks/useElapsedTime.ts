import { useEffect, useState } from "react";
import { formatElapsed } from "../lib/format";

export function useElapsedTime(startTime: number | null): string {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    if (!startTime) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [startTime]);

  if (!startTime) return "00:00";
  return formatElapsed(now - startTime);
}
