"use client";

import Link from "next/link";
import { useTheme } from "next-themes";
import { useState, useEffect } from "react";
import { Sun, Moon, Github, Sparkles } from "lucide-react";

export function Header() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => setMounted(true), []);

  return (
    <header className="sticky top-0 z-50 w-full border-b glass">
      <div className="container mx-auto flex h-16 items-center justify-between px-4">
        <Link href="/" className="flex items-center gap-2 group">
          <Sparkles className="h-5 w-5 text-primary group-hover:rotate-12 transition-transform" />
          <span className="font-mono text-lg font-bold gradient-text">
            Tools
          </span>
        </Link>

        <nav className="hidden md:flex items-center gap-6 text-sm">
          <Link
            href="/scan"
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            Scanner
          </Link>
          <Link
            href="/image/compress"
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            Image
          </Link>
          <Link
            href="/pdf/merge"
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            PDF
          </Link>
        </nav>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            className="p-2 rounded-md hover:bg-muted transition-colors"
            aria-label="Toggle theme"
          >
            {mounted && theme === "dark" ? (
              <Sun className="h-4 w-4" />
            ) : (
              <Moon className="h-4 w-4" />
            )}
          </button>
          <a
            href="https://github.com/asepharyana/asepharyana-hub"
            target="_blank"
            rel="noopener noreferrer"
            className="p-2 rounded-md hover:bg-muted transition-colors"
            aria-label="GitHub"
          >
            <Github className="h-4 w-4" />
          </a>
        </div>
      </div>
    </header>
  );
}