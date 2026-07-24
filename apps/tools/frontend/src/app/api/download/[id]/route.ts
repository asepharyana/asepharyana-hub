import { NextRequest, NextResponse } from "next/server";

const RUST_GATEWAY = process.env.RUST_GATEWAY_URL || "http://localhost:3001";

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  try {
    const response = await fetch(`${RUST_GATEWAY}/api/download/${id}`);

    if (!response.ok) {
      const data = await response.json().catch(() => null);
      return NextResponse.json(
        data ?? { error: "Download failed" },
        { status: response.status },
      );
    }

    // Stream the file back
    const blob = await response.blob();
    const contentType =
      response.headers.get("content-type") || "application/octet-stream";
    const contentDisposition =
      response.headers.get("content-disposition") ||
      "attachment; filename=\"result\"";

    return new NextResponse(blob, {
      headers: {
        "Content-Type": contentType,
        "Content-Disposition": contentDisposition,
      },
    });
  } catch (error) {
    console.error("Download proxy error:", error);
    return NextResponse.json(
      { error: "Failed to download file" },
      { status: 500 },
    );
  }
}