export function Footer() {
  return (
    <footer className="border-t py-6 mt-auto">
      <div className="container mx-auto px-4 flex flex-col md:flex-row items-center justify-between gap-4 text-sm text-muted-foreground">
        <p>
          &copy; {new Date().getFullYear()} Asep Haryana Saputra. All rights
          reserved.
        </p>
        <p className="flex items-center gap-1">
          Powered by{" "}
          <span className="font-mono text-primary">Rust + Next.js</span>
        </p>
      </div>
    </footer>
  );
}