import { NextRequest, NextResponse } from "next/server";

const RUST_GATEWAY = process.env.RUST_GATEWAY_URL || "http://localhost:3001";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  try {
    const response = await fetch(`${RUST_GATEWAY}/api/job/${id}`);
    const data = await response.json();

    if (!response.ok) {
      return NextResponse.json(data, { status: response.status });
    }

    return NextResponse.json(data);
  } catch (error) {
    console.error("Job status proxy error:", error);
    return NextResponse.json(
      { error: "Failed to fetch job status" },
      { status: 500 },
    );
  }
}