import type { Metadata } from "next";
import { ThemeProvider } from "next-themes";

import "./globals.css";
import { Header } from "@/components/tools/header";
import { Footer } from "@/components/tools/footer";

export const metadata: Metadata = {
  title: "Tools — Asep Haryana",
  description:
    "Self-hosted document scanner, image tools & PDF tools. No upload to third-party servers.",
  manifest: "/manifest.json",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="id" suppressHydrationWarning>
      <body className="min-h-screen flex flex-col antialiased">
        <ThemeProvider
          attribute="class"
          defaultTheme="dark"
          enableSystem
          disableTransitionOnChange
        >
          <Header />
          <main className="flex-1">{children}</main>
          <Footer />
        </ThemeProvider>
      </body>
    </html>
  );
}