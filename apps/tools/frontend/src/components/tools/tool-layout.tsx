"use client";

import type { LucideIcon } from "lucide-react";

interface ToolLayoutProps {
  title: string;
  description: string;
  icon: LucideIcon;
  phase: number;
  children: React.ReactNode;
}

export function ToolLayout({
  title,
  description,
  icon: Icon,
  phase,
  children,
}: ToolLayoutProps) {
  const isAvailable = phase === 1;

  return (
    <div className="container mx-auto px-4 py-8 max-w-3xl">
      <div className="flex items-center gap-3 mb-8">
        <div className="p-2 rounded-lg bg-primary/10 text-primary">
          <Icon className="h-6 w-6" />
        </div>
        <div>
          <h1 className="text-2xl font-bold">{title}</h1>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
      </div>

      {!isAvailable ? (
        <div className="p-8 rounded-xl border glass text-center space-y-3">
          <p className="text-lg font-medium">Coming Soon</p>
          <p className="text-sm text-muted-foreground">
            Tool ini sedang dalam pengembangan dan akan tersedia di fase
            berikutnya.
          </p>
        </div>
      ) : (
        children
      )}
    </div>
  );
}