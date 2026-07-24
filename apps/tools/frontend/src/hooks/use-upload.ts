"use client";

import { useState, useCallback } from "react";

interface UploadResult {
  job_id: string;
  ws_url: string;
  status: string;
}

interface UseUploadOptions {
  tool: string;
  options?: Record<string, unknown>;
}

export function useUpload({ tool, options }: UseUploadOptions) {
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<UploadResult | null>(null);
  const abortRef = useState<AbortController | null>(null);

  const upload = useCallback(
    async (file: File): Promise<UploadResult | null> => {
      setIsUploading(true);
      setError(null);
      setResult(null);

      try {
        const formData = new FormData();
        formData.append("file", file);
        formData.append("tool", tool);
        if (options) {
          formData.append("options", JSON.stringify(options));
        }

        const controller = new AbortController();
        abortRef[1](controller);

        const response = await fetch("/api/upload", {
          method: "POST",
          body: formData,
          signal: controller.signal,
        });

        if (!response.ok) {
          const errData = await response.json().catch(() => null);
          throw new Error(
            errData?.error ?? `Upload failed: ${response.status}`,
          );
        }

        const data: UploadResult = await response.json();
        setResult(data);
        return data;
      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          return null;
        }
        const msg = err instanceof Error ? err.message : "Upload failed";
        setError(msg);
        throw err;
      } finally {
        setIsUploading(false);
      }
    },
    [tool, options],
  );

  const cancel = useCallback(() => {
    abortRef[1]?.abort();
    setIsUploading(false);
  }, [abortRef[1]]);

  return { upload, cancel, isUploading, error, result };
}