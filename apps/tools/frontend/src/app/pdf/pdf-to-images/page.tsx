"use client";

import { useState, useCallback } from "react";
import { ImageIcon } from "lucide-react";
import { ToolLayout } from "@/components/tools/tool-layout";
import { UploadZone } from "@/components/tools/upload-zone";
import { useUpload } from "@/hooks/use-upload";
import { useJobStatus } from "@/hooks/use-job-status";
import { ProgressBar } from "@/components/tools/progress-bar";
import { ResultPreview } from "@/components/tools/result-preview";

type PageState = "upload" | "processing" | "result" | "error";

export default function PdfToImagesPage() {
  const [pageState, setPageState] = useState<PageState>("upload");
  const [jobId, setJobId] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const { upload } = useUpload({
    tool: "pdf-pdf-to-images",
    options: { quality: 80 },
  });

  const handleComplete = useCallback(() => setPageState("result"), []);
  const handleError = useCallback(
    (err: string) => {
      setErrorMsg(err);
      setPageState("error");
    },
    [],
  );

  const { progress, stage, message, status, result } = useJobStatus(jobId, {
    onComplete: handleComplete,
    onError: handleError,
  });

  const handleUpload = useCallback(
    async (file: File) => {
      setErrorMsg(null);
      const res = await upload(file);
      if (res) {
        setJobId(res.job_id);
        setPageState("processing");
      }
    },
    [upload],
  );

  const handleRetry = useCallback(() => {
    setPageState("upload");
    setJobId(null);
    setErrorMsg(null);
  }, []);

  return (
    <ToolLayout
      title="PDF to Images"
      description="Convert tiap halaman ke gambar"
      icon={ImageIcon}
      phase={2}
    >
      {pageState === "upload" && (
        <UploadZone
          accept="application/pdf"
          tool="pdf-pdf-to-images"
          onUpload={handleUpload}
        />
      )}

      {pageState === "processing" && jobId && (
        <ProgressBar
          progress={progress}
          stage={stage}
          message={message}
          status={status}
          onRetry={handleRetry}
        />
      )}

      {pageState === "result" && result && (
        <ResultPreview
          result={result}
          onProcessAnother={handleRetry}
        />
      )}

      {pageState === "error" && (
        <div className="p-6 rounded-xl border border-destructive/20 bg-destructive/5 text-center">
          <p className="text-destructive font-medium mb-4">
            {errorMsg || "Terjadi kesalahan"}
          </p>
          <button
            onClick={handleRetry}
            className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90"
          >
            Coba Lagi
          </button>
        </div>
      )}
    </ToolLayout>
  );
}
