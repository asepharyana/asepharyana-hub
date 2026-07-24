"use client";

import {
  Scan,
  ImageDown,
  Crop,
  Repeat,
  Shrink,
  Merge,
  Split,
  Images,
  FileImage,
  Video,
  Music,
  Scissors,
  Film,
  Mic,
} from "lucide-react";
import { ToolCard } from "./tool-card";

interface ToolDefinition {
  id: string;
  title: string;
  description: string;
  icon: typeof Scan;
  href: string;
  phase: number;
}

const tools: ToolDefinition[] = [
  {
    id: "scan",
    title: "Document Scanner",
    description:
      "Foto dokumen pake HP — auto-detect tepi, lurusin, enhance, OCR. Output searchable PDF.",
    icon: Scan,
    href: "/scan",
    phase: 1,
  },
  {
    id: "image-compress",
    title: "Compress Image",
    description: "Kecilin ukuran JPEG/PNG/WebP tanpa ilangin kualitas. Atur quality %.",
    icon: ImageDown,
    href: "/image/compress",
    phase: 1,
  },
  {
    id: "image-resize",
    title: "Resize Image",
    description:
      "Ubah dimensi gambar. Preset ukuran social media, aspect ratio lock.",
    icon: Crop,
    href: "/image/resize",
    phase: 1,
  },
  {
    id: "image-convert",
    title: "Convert Image",
    description: "Convert HEIC→JPEG, PNG→WebP, SVG→PNG, dan banyak lagi.",
    icon: Repeat,
    href: "/image/convert",
    phase: 1,
  },
  {
    id: "remove-bg",
    title: "Remove Background",
    description: "Hapus latar belakang foto otomatis pake AI. Download PNG transparan.",
    icon: Shrink,
    href: "/image/remove-bg",
    phase: 2,
  },
  {
    id: "pdf-merge",
    title: "Merge PDF",
    description: "Gabung beberapa file PDF jadi satu. Drag to reorder halaman.",
    icon: Merge,
    href: "/pdf/merge",
    phase: 2,
  },
  {
    id: "pdf-split",
    title: "Split PDF",
    description: "Ekstrak halaman tertentu dari PDF. Pilih via thumbnail atau range.",
    icon: Split,
    href: "/pdf/split",
    phase: 2,
  },
  {
    id: "images-to-pdf",
    title: "Images to PDF",
    description: "Kumpulan foto jadi 1 file PDF. Atur ukuran halaman dan margin.",
    icon: Images,
    href: "/pdf/images-to-pdf",
    phase: 2,
  },
  {
    id: "pdf-compress",
    title: "Compress PDF",
    description: "Kecilin ukuran PDF dengan kompresi embedded images.",
    icon: FileImage,
    href: "/pdf/compress",
    phase: 2,
  },
  {
    id: "video-compress",
    title: "Compress Video",
    description: "Turunin bitrate & resolusi video. H.264/H.265/VP9.",
    icon: Video,
    href: "/video/compress",
    phase: 3,
  },
  {
    id: "audio-extract",
    title: "Extract Audio",
    description: "Ambil audio dari file video. MP3, AAC, WAV, FLAC.",
    icon: Music,
    href: "/video/audio-extract",
    phase: 3,
  },
  {
    id: "video-trim",
    title: "Trim Video",
    description: "Potong segmen video. Set start/end via timeline.",
    icon: Scissors,
    href: "/video/trim",
    phase: 3,
  },
  {
    id: "gif-maker",
    title: "GIF Maker",
    description: "Convert video segment ke animated GIF. Atur FPS, resolusi, dither.",
    icon: Film,
    href: "/video/gif-maker",
    phase: 3,
  },
  {
    id: "audio-convert",
    title: "Audio Convert",
    description: "Convert audio antar format. MP3, WAV, FLAC, AAC, OGG.",
    icon: Mic,
    href: "/audio/convert",
    phase: 3,
  },
];

export function ToolGrid() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
      {tools.map((tool, index) => (
        <ToolCard
          key={tool.id}
          title={tool.title}
          description={tool.description}
          icon={tool.icon}
          href={tool.href}
          phase={tool.phase}
          index={index}
        />
      ))}
    </div>
  );
}