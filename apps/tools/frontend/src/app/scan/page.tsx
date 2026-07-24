"use client";

import { useState, useCallback } from "react";
import { Scan } from "lucide-react";
import { UploadZone } from "@/components/tools/upload-zone";
import { ProgressBar } from "@/components/tools/progress-bar";
import { ResultPreview } from "@/components/tools/result-preview";
import { useUpload } from "@/hooks/use-upload";
import { useJobStatus } from "@/hooks/use-job-status";

type PageState = "upload" | "processing" | "result" | "error";

export default function ScanPage() {
  const [pageState, setPageState] = useState<PageState>("upload");
  const [jobId, setJobId] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const { upload, isUploading } = useUpload({
    tool: "scan",
    options: { ocr: true, enhance: true, output_format: "pdf", dpi: 300 },
  });

  const handleComplete = useCallback(() => {
    setPageState("result");
  }, []);

  const handleError = useCallback((err: string) => {
    setErrorMsg(err);
    setPageState("error");
  }, []);

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
    <div className="container mx-auto px-4 py-8 max-w-3xl">
      <div className="flex items-center gap-3 mb-8">
        <div className="p-2 rounded-lg bg-primary/10 text-primary">
          <Scan className="h-6 w-6" />
        </div>
        <div>
          <h1 className="text-2xl font-bold">Document Scanner</h1>
          <p className="text-sm text-muted-foreground">
            Foto dokumen pake HP — auto-detect, lurusin, enhance, OCR
          </p>
        </div>
      </div>

      {pageState === "upload" && (
        <div className="space-y-6">
          <UploadZone
            accept="image/*"
            tool="scan"
            onUpload={handleUpload}
            maxSizeMB={50}
          />

          {/* Options info */}
          <div className="p-4 rounded-lg border glass text-sm text-muted-foreground">
            <p className="font-medium text-foreground mb-2">Scan Options</p>
            <ul className="space-y-1">
              <li>• OCR: Enabled (English + Indonesian)</li>
              <li>• Output: Searchable PDF</li>
              <li>• DPI: 300</li>
              <li>• Auto-enhance: On</li>
            </ul>
          </div>
        </div>
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
          result={{
            download_url: result.download_url,
            file_size: result.file_size,
            file_name: result.file_name,
            preview_url: result.preview_url,
          }}
          ocrText={result.ocr_text}
          onProcessAnother={handleRetry}
        />
      )}

      {pageState === "error" && (
        <div className="p-6 rounded-xl border border-destructive/20 bg-destructive/5 text-center space-y-4">
          <p className="text-destructive font-medium">
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
    </div>
  );
}