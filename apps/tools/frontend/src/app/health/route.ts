import { NextResponse } from "next/server";

const RUST_GATEWAY = process.env.RUST_GATEWAY_URL || "http://localhost:3001";

export async function GET() {
  try {
    const response = await fetch(`${RUST_GATEWAY}/health`, {
      signal: AbortSignal.timeout(5000),
    });
    const data = await response.json();
    return NextResponse.json(data);
  } catch {
    return NextResponse.json(
      { status: "error", message: "Gateway unreachable" },
      { status: 503 },
    );
  }
}