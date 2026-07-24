"use client";

import { useState, useRef, useCallback } from "react";

interface PreviewBeforeAfterProps {
  originalUrl: string;
  processedUrl: string;
  originalSize?: number;
  processedSize?: number;
}

export function PreviewBeforeAfter({
  originalUrl,
  processedUrl,
  originalSize,
  processedSize,
}: PreviewBeforeAfterProps) {
  const [sliderPos, setSliderPos] = useState(50);
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  const handleMove = useCallback(
    (clientX: number) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const x = Math.max(0, Math.min(clientX - rect.left, rect.width));
      setSliderPos((x / rect.width) * 100);
    },
    [],
  );

  const handleMouseDown = () => {
    isDragging.current = true;
  };

  const handleMouseUp = () => {
    isDragging.current = false;
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging.current) return;
    handleMove(e.clientX);
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    handleMove(e.touches[0].clientX);
  };

  const formatSize = (bytes?: number) => {
    if (!bytes) return "";
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  };

  return (
    <div className="space-y-3">
      <div
        ref={containerRef}
        className="relative rounded-lg overflow-hidden select-none cursor-ew-resize aspect-[4/3] max-h-96 bg-muted"
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseUp}
        onTouchMove={handleTouchMove}
      >
        {/* Processed (full) */}
        <img
          src={processedUrl}
          alt="Processed"
          className="absolute inset-0 w-full h-full object-contain"
          draggable={false}
        />

        {/* Original (clipped) */}
        <div
          className="absolute inset-0 overflow-hidden"
          style={{ width: `${sliderPos}%` }}
        >
          <img
            src={originalUrl}
            alt="Original"
            className="absolute top-0 left-0 w-full h-full object-contain"
            style={{
              width: `${100 / (sliderPos / 100)}%`,
              maxWidth: "none",
            }}
            draggable={false}
          />
        </div>

        {/* Slider */}
        <div
          className="absolute top-0 bottom-0 w-0.5 bg-white shadow-lg z-10"
          style={{ left: `${sliderPos}%` }}
        >
          <div className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-8 h-8 rounded-full bg-white shadow-lg flex items-center justify-center text-xs text-gray-800 font-bold">
            ⟷
          </div>
        </div>

        {/* Labels */}
        <div className="absolute top-2 left-2 px-2 py-1 bg-black/60 text-white text-xs rounded backdrop-blur-sm">
          Original
        </div>
        <div className="absolute top-2 right-2 px-2 py-1 bg-black/60 text-white text-xs rounded backdrop-blur-sm">
          Processed
        </div>
      </div>

      {(originalSize || processedSize) && (
        <div className="flex items-center justify-center gap-4 text-sm text-muted-foreground">
          {originalSize && (
            <span>
              Original:{" "}
              <span className="text-foreground font-medium">
                {formatSize(originalSize)}
              </span>
            </span>
          )}
          {processedSize && (
            <span>
              Processed:{" "}
              <span className="text-green-500 font-medium">
                {formatSize(processedSize)}
              </span>
            </span>
          )}
        </div>
      )}
    </div>
  );
}