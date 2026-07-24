import { ToolGrid } from "@/components/tools/tool-grid";

export default function HomePage() {
  return (
    <div className="container mx-auto px-4 py-12">
      {/* Hero */}
      <section className="text-center mb-16">
        <h1 className="text-4xl md:text-5xl font-bold mb-4">
          <span className="gradient-text">Tools</span>
        </h1>
        <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
          Self-hosted document scanner, image tools & PDF tools.
          <br />
          Semua proses di backend — cepat, hemat,{" "}
          <span className="text-primary font-semibold">privacy first</span>.
        </p>
        <div className="flex items-center justify-center gap-4 mt-6 text-sm text-muted-foreground">
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-green-500" />
            Rust + WASM
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-primary" />
            No upload to 3rd party
          </span>
          <span className="flex items-center gap-1">
            <span className="w-2 h-2 rounded-full bg-amber-500" />
            Auto-delete 1 jam
          </span>
        </div>
      </section>

      {/* Tools Grid */}
      <section>
        <div className="flex items-center justify-between mb-8">
          <h2 className="text-2xl font-bold">All Tools</h2>
          <span className="text-sm text-muted-foreground font-mono">
            14 tools
          </span>
        </div>
        <ToolGrid />
      </section>

      {/* Privacy Note */}
      <section className="mt-16 p-6 rounded-xl border glass text-center">
        <h2 className="text-lg font-semibold mb-2">🔒 Privacy First</h2>
        <p className="text-sm text-muted-foreground max-w-xl mx-auto">
          Semua file diproses di server kami dan{" "}
          <span className="text-primary font-medium">
            otomatis dihapus setelah 1 jam
          </span>
          . Tidak ada data yang dikirim ke pihak ketiga. Source code
          open-source di GitHub.
        </p>
      </section>
    </div>
  );
}