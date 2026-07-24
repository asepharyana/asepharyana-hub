"use client";

import { useState } from "react";
import { Download, RefreshCw, FileText, Copy } from "lucide-react";

interface ResultInfo {
  download_url: string;
  file_size: number;
  file_name: string;
  preview_url?: string;
}

interface ResultPreviewProps {
  result: ResultInfo;
  ocrText?: string;
  onProcessAnother: () => void;
}

export function ResultPreview({
  result,
  ocrText,
  onProcessAnother,
}: ResultPreviewProps) {
  const [copied, setCopied] = useState(false);
  const [autoDownload, setAutoDownload] = useState(false);

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  };

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(
        `${window.location.origin}${result.download_url}`,
      );
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback
    }
  };

  return (
    <div className="space-y-4 p-6 rounded-xl border glass">
      <div className="flex items-center gap-3">
        <div className="p-3 rounded-lg bg-primary/10 text-primary">
          <FileText className="h-6 w-6" />
        </div>
        <div className="min-w-0 flex-1">
          <p className="font-medium truncate">{result.file_name}</p>
          <p className="text-sm text-muted-foreground">
            {formatSize(result.file_size)}
          </p>
        </div>
      </div>

      {result.preview_url && (
        <div className="relative rounded-lg overflow-hidden bg-muted aspect-[4/3] max-h-80">
          <img
            src={result.preview_url}
            alt="Preview"
            className="w-full h-full object-contain"
          />
        </div>
      )}

      {ocrText && (
        <details className="text-sm">
          <summary className="cursor-pointer text-muted-foreground hover:text-foreground">
            OCR Text
          </summary>
          <pre className="mt-2 p-3 bg-muted rounded-lg text-xs overflow-auto max-h-32">
            {ocrText}
          </pre>
        </details>
      )}

      <div className="flex flex-wrap items-center gap-3">
        <a
          href={result.download_url}
          download
          className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 transition-opacity font-medium"
        >
          <Download className="h-4 w-4" />
          Download
        </a>

        {typeof navigator !== "undefined" && navigator.clipboard && (
          <button
            onClick={handleCopy}
            className="inline-flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted transition-colors text-sm"
          >
            <Copy className="h-4 w-4" />
            {copied ? "Copied!" : "Copy Link"}
          </button>
        )}

        <button
          onClick={onProcessAnother}
          className="inline-flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted transition-colors text-sm ml-auto"
        >
          <RefreshCw className="h-4 w-4" />
          Process Another
        </button>
      </div>

      <label className="flex items-center gap-2 text-sm text-muted-foreground cursor-pointer">
        <input
          type="checkbox"
          checked={autoDownload}
          onChange={(e) => setAutoDownload(e.target.checked)}
          className="rounded"
        />
        Auto-download on complete
      </label>
    </div>
  );
}