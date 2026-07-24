"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import type { LucideIcon } from "lucide-react";

interface ToolCardProps {
  title: string;
  description: string;
  icon: LucideIcon;
  href: string;
  phase: number;
  index: number;
}

export function ToolCard({
  title,
  description,
  icon: Icon,
  href,
  phase,
  index,
}: ToolCardProps) {
  const isAvailable = phase === 1;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, delay: index * 0.05 }}
    >
      <Link
        href={isAvailable ? href : "#"}
        className={`group block p-6 rounded-xl border transition-all duration-200 ${
          isAvailable
            ? "hover:border-primary hover:shadow-lg hover:shadow-primary/5 cursor-pointer"
            : "opacity-50 cursor-not-allowed"
        } glass`}
        onClick={(e) => {
          if (!isAvailable) e.preventDefault();
        }}
      >
        <div className="flex items-start gap-4">
          <div className="p-3 rounded-lg bg-primary/10 text-primary shrink-0">
            <Icon className="h-6 w-6" />
          </div>
          <div className="min-w-0">
            <h3 className="font-semibold mb-1 group-hover:text-primary transition-colors">
              {title}
            </h3>
            <p className="text-sm text-muted-foreground line-clamp-2">
              {description}
            </p>
            <span
              className={`inline-block mt-3 text-xs px-2 py-0.5 rounded-full ${
                isAvailable
                  ? "bg-primary/10 text-primary"
                  : "bg-muted text-muted-foreground"
              }`}
            >
              {isAvailable ? "Available" : "Coming Soon"}
            </span>
          </div>
        </div>
      </Link>
    </motion.div>
  );
}