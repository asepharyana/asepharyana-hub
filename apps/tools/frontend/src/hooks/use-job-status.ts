"use client";

import { useState, useEffect, useRef, useCallback } from "react";

export type JobStatusType = "queued" | "processing" | "completed" | "failed";

interface JobProgress {
  type: "progress" | "complete" | "error" | "status" | "ping";
  job_id: string;
  status: JobStatusType;
  progress: number;
  stage: string;
  message: string;
  result?: {
    download_url: string;
    file_name: string;
    file_size: number;
    preview_url?: string;
    ocr_text?: string;
  };
  error?: string;
}

interface UseJobStatusOptions {
  onComplete?: (result: JobProgress["result"]) => void;
  onError?: (error: string) => void;
}

export function useJobStatus(jobId: string | null, options?: UseJobStatusOptions) {
  const [progress, setProgress] = useState(0);
  const [stage, setStage] = useState("queued");
  const [message, setMessage] = useState("");
  const [status, setStatus] = useState<JobStatusType>("queued");
  const [result, setResult] = useState<JobProgress["result"] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const retryCount = useRef(0);
  const maxRetries = 3;

  const connect = useCallback(() => {
    if (!jobId) return;

    // Connect directly to Rust gateway WebSocket (not via Next.js)
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const host = "localhost:3001"; // Rust gateway - wss for production
    const url = `${protocol}//${host}/api/job/${jobId}/ws`;

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      retryCount.current = 0;
    };

    ws.onmessage = (event) => {
      try {
        const data: JobProgress = JSON.parse(event.data);

        if (data.type === "ping") return;

        setStatus(data.status);
        setProgress(data.progress);
        setStage(data.stage);
        setMessage(data.message);

        if (data.type === "complete") {
          setResult(data.result ?? null);
          options?.onComplete?.(data.result);
        }

        if (data.type === "error") {
          setError(data.error ?? "Unknown error");
          options?.onError?.(data.error ?? "Unknown error");
        }
      } catch {
        // Ignore parse errors
      }
    };

    ws.onclose = () => {
      if (retryCount.current < maxRetries) {
        retryCount.current++;
        setTimeout(connect, 1000 * retryCount.current);
      }
    };

    ws.onerror = () => {
      ws.close();
    };
  }, [jobId, options]);

  useEffect(() => {
    connect();
    return () => {
      wsRef.current?.close();
    };
  }, [connect]);

  return { progress, stage, message, status, result, error };
}