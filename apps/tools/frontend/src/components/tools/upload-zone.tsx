"use client";

import { useState, useRef, useCallback, type DragEvent } from "react";
import { Upload, File, X, Image as ImageIcon } from "lucide-react";
import { cn } from "@/lib/utils";

interface UploadZoneProps {
  accept?: string;
  maxSizeMB?: number;
  multiple?: boolean;
  tool: string;
  onUpload: (file: File) => void;
  onCancel?: () => void;
}

export function UploadZone({
  accept = "image/*,.pdf",
  maxSizeMB = 50,
  multiple = false,
  tool,
  onUpload,
  onCancel,
}: UploadZoneProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const validateFile = useCallback(
    (file: File): string | null => {
      const maxBytes = maxSizeMB * 1024 * 1024;
      if (file.size > maxBytes) {
        return `File terlalu besar: ${(file.size / 1024 / 1024).toFixed(1)}MB (max ${maxSizeMB}MB)`;
      }
      return null;
    },
    [maxSizeMB],
  );

  const handleDrop = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDragging(false);
      const file = e.dataTransfer.files[0];
      if (!file) return;
      const err = validateFile(file);
      if (err) {
        setError(err);
        return;
      }
      setError(null);
      setSelectedFile(file);
    },
    [validateFile],
  );

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;
      const err = validateFile(file);
      if (err) {
        setError(err);
        return;
      }
      setError(null);
      setSelectedFile(file);
    },
    [validateFile],
  );

  const handleUpload = useCallback(async () => {
    if (!selectedFile) return;
    setIsUploading(true);
    setError(null);
    try {
      await onUpload(selectedFile);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Upload failed");
    } finally {
      setIsUploading(false);
    }
  }, [selectedFile, onUpload]);

  const formatSize = (bytes: number) => {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  };

  return (
    <div className="space-y-4">
      <div
        onDragOver={(e) => {
          e.preventDefault();
          setIsDragging(true);
        }}
        onDragLeave={() => setIsDragging(false)}
        onDrop={handleDrop}
        onClick={() => !selectedFile && inputRef.current?.click()}
        className={cn(
          "relative border-2 border-dashed rounded-xl p-8 md:p-12 text-center transition-all duration-200 cursor-pointer",
          isDragging
            ? "border-primary bg-primary/5"
            : "border-border hover:border-primary/50",
          selectedFile && "border-solid border-primary/30",
        )}
      >
        <input
          ref={inputRef}
          type="file"
          accept={accept}
          multiple={multiple}
          className="hidden"
          onChange={handleFileSelect}
        />

        {!selectedFile ? (
          <div className="space-y-4">
            <div className="flex justify-center">
              <Upload className="h-12 w-12 text-muted-foreground" />
            </div>
            <div>
              <p className="font-medium">
                Drag & drop file here, or click to browse
              </p>
              <p className="text-sm text-muted-foreground mt-1">
                Max {maxSizeMB}MB per file
              </p>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex items-center justify-center gap-3">
              <ImageIcon className="h-8 w-8 text-primary" />
              <div className="text-left">
                <p className="font-medium truncate max-w-[300px]">
                  {selectedFile.name}
                </p>
                <p className="text-sm text-muted-foreground">
                  {formatSize(selectedFile.size)}
                </p>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setSelectedFile(null);
                  setError(null);
                }}
                className="p-1 hover:bg-muted rounded"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <button
              onClick={(e) => {
                e.stopPropagation();
                handleUpload();
              }}
              disabled={isUploading}
              className="px-6 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-90 transition-opacity disabled:opacity-50 font-medium"
            >
              {isUploading ? "Uploading..." : "Process"}
            </button>
          </div>
        )}
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-lg text-sm text-destructive">
          {error}
        </div>
      )}
    </div>
  );
}