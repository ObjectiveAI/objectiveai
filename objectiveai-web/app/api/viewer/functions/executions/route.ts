import { NextRequest, NextResponse } from "next/server";
import { pushEvent } from "@/lib/viewer-events";

export async function POST(req: NextRequest) {
  const payload = await req.json();
  pushEvent({
    kind: "functions-executions",
    payload,
    timestamp: Date.now(),
  });
  return NextResponse.json({ ok: true });
}
