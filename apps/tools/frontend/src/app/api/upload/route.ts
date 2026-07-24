import { NextRequest, NextResponse } from "next/server";

const RUST_GATEWAY = process.env.RUST_GATEWAY_URL || "http://localhost:3001";

export async function POST(request: NextRequest) {
  try {
    const formData = await request.formData();
    const file = formData.get("file");
    const tool = formData.get("tool");
    const options = formData.get("options");

    if (!file || !tool) {
      return NextResponse.json(
        { error: "Missing file or tool parameter" },
        { status: 400 },
      );
    }

    // Forward to Rust gateway
    const gatewayForm = new FormData();
    gatewayForm.append("file", file);
    gatewayForm.append("tool", tool as string);
    if (options) {
      gatewayForm.append("options", options as string);
    }

    const response = await fetch(`${RUST_GATEWAY}/api/upload`, {
      method: "POST",
      body: gatewayForm,
    });

    const data = await response.json();

    if (!response.ok) {
      return NextResponse.json(data, { status: response.status });
    }

    return NextResponse.json(data, { status: 202 });
  } catch (error) {
    console.error("Upload proxy error:", error);
    return NextResponse.json(
      { error: "Failed to process upload" },
      { status: 500 },
    );
  }
}