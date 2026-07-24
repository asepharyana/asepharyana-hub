import { NextResponse } from "next/server";

// WebSocket is handled directly by the client connecting to the Rust gateway.
// Next.js App Router cannot proxy WebSocket connections in route handlers.
// The client-side useJobStatus hook connects directly to ws://localhost:3001/api/job/{id}/ws
// In production, configure the WebSocket to connect to wss://tools.asepharyana.my.id/api/job/{id}/ws
export function GET() {
  return NextResponse.json(
    {
      note: "WebSocket connections go directly to the Rust gateway",
      ws_url:
        process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:3001/api/job/{id}/ws",
    },
  );
}