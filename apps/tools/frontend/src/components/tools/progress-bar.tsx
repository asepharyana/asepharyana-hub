"use client";

import { motion } from "framer-motion";
import { cn } from "@/lib/utils";

export type JobStatus = "queued" | "processing" | "completed" | "failed";

interface ProgressBarProps {
  progress: number;
  stage: string;
  message: string;
  status: JobStatus;
  onRetry?: () => void;
}

const stageLabels: Record<string, string> = {
  preprocess: "Memuat gambar...",
  edge_detection: "Mendeteksi tepi dokumen...",
  corner_detection: "Mencari sudut dokumen...",
  warp: "Meluruskan perspektif...",
  shadow_removal: "Menghilangkan bayangan...",
  binarization: "Mengubah ke hitam-putih...",
  deskew: "Meluruskan teks...",
  enhance: "Mengoptimalkan kontras...",
  ocr: "Membaca teks...",
  pdf_generation: "Membuat PDF...",
  complete: "Selesai!",
};

function getStageLabel(stage: string): string {
  return stageLabels[stage] || stage;
}

export function ProgressBar({
  progress,
  stage,
  message,
  status,
  onRetry,
}: ProgressBarProps) {
  const barColor =
    status === "completed"
      ? "bg-green-500"
      : status === "failed"
        ? "bg-destructive"
        : "bg-primary";

  const statusBadge =
    status === "processing" ? (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-600 dark:text-amber-400">
        <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse" />
        Processing
      </span>
    ) : status === "completed" ? (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-500/10 text-green-600 dark:text-green-400">
        Completed
      </span>
    ) : status === "failed" ? (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-destructive/10 text-destructive">
        Failed
      </span>
    ) : null;

  return (
    <div className="space-y-3 p-6 rounded-xl border glass">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">{statusBadge}</span>
        <span className="text-sm font-mono text-muted-foreground">
          {progress}%
        </span>
      </div>

      <div className="relative h-2 bg-muted rounded-full overflow-hidden">
        <motion.div
          className={cn("absolute inset-y-0 left-0 rounded-full", barColor)}
          initial={{ width: 0 }}
          animate={{ width: `${progress}%` }}
          transition={{ duration: 0.5, ease: "easeOut" }}
        />
      </div>

      <div>
        <p className="text-sm font-medium">
          {getStageLabel(stage)}
        </p>
        {message && (
          <p className="text-xs text-muted-foreground mt-0.5">{message}</p>
        )}
      </div>

      {status === "failed" && onRetry && (
        <button
          onClick={onRetry}
          className="text-sm text-primary hover:underline"
        >
          Coba lagi
        </button>
      )}
    </div>
  );
}