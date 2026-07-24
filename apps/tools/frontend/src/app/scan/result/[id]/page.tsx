"use client";

import { useParams } from "next/navigation";
import { useJobStatus } from "@/hooks/use-job-status";
import { ProgressBar } from "@/components/tools/progress-bar";
import { ResultPreview } from "@/components/tools/result-preview";

export default function ScanResultPage() {
  const params = useParams();
  const id = params.id as string;

  const { progress, stage, message, status, result, error } = useJobStatus(id, {
    onComplete: () => {},
    onError: () => {},
  });

  return (
    <div className="container mx-auto px-4 py-8 max-w-3xl">
      {status === "processing" && (
        <ProgressBar progress={progress} stage={stage} message={message} status={status} />
      )}
      {status === "completed" && result && (
        <ResultPreview result={result} onProcessAnother={() => window.location.href = "/scan"} />
      )}
      {status === "failed" && (
        <div className="p-6 rounded-xl border glass text-center">
          <p className="text-destructive font-medium">{error || "Processing failed"}</p>
        </div>
      )}
    </div>
  );
}
