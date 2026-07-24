# Tools — Document Scanner & Media Processing Hub

Self-hosted, no-install document scanner dan media processing tools yang jalan di browser. Alternatif dari CamScanner, ilovepdf, compressjpeg — tanpa upload ke pihak ketiga.

## Visi

Satu platform dengan tools manipulasi file yang **beneran dipake orang setiap hari**. Semua proses di backend Rust — cepat, hemat memory, ga perlu install software.

## Fitur Utama

### Phase 1 — Document Scanner (Prioritas)
- Foto dokumen pake HP → auto-detect tepi → lurusin (perspective correction)
- Enhance: iluminasi merata, contrast, sharpen, B&W
- OCR → searchable PDF (teks bisa di-copy, dicari)
- Batch: multi-page → satu PDF
- Fallback crop manual (kalau auto-detect gagal)

### Phase 2 — Image Tools
- Compress JPEG/PNG/WebP (lossy + lossless, atur kualitas %)
- Resize batch (atur dimensi, semua foto disamain)
- Convert format (HEIC→JPEG, PNG→WebP, SVG→PNG)
- Remove background (ONNX model, Rust runtime)

### Phase 3 — PDF Tools
- Merge PDF (gabung file)
- Split PDF (ekstrak halaman tertentu)
- Images→PDF (kumpulan foto jadi 1 file)
- PDF→Images (tiap halaman jadi gambar)
- PDF compress (turunkin kualitas embedded images)

### Phase 4 — Video/Audio Tools
- Compress video (bitrate + resolusi)
- Extract audio (MP4→MP3)
- Trim/crop
- GIF maker
- Audio convert + trim

## Target User

Orang yang:
- Punya HP/PC, paham teknologi dasar (buka browser, upload file)
- Butuh scan dokumen tanpa install aplikasi
- Butuh kompres file buat kirim WA/email
- Butuh manipulasi PDF sesekali
- Peduli privasi — ga mau upload file ke server pihak ketiga

## Prinsip Desain

1. **Satu task selesai dalam <5 detik** — ga ada loading lama
2. **Drag & drop + preview** — liat hasil sebelum download
3. **Progress realtime** via WebSocket — tau lagi di tahap mana
4. **Batch processing** — banyak file, satu klik
5. **Privasi first** — file otomatis dihapus setelah 1 jam
6. **WASM fallback** — tools ringan jalan di client (tanpa upload)

## Domain & Branding

- **Domain**: `tools.asepharyana.my.id` | `tools.asepharyana.web.id`
- **Design**: Twilight Terminal theme (sama kaya portfolio), konsisten visual
- **Dashboard**: Link dari hub dashboard → tools stats (total files processed, storage used)
