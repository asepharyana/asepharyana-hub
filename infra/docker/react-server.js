import { existsSync, readFileSync } from "node:fs"
import { resolve, sep } from "node:path"

const distDir = resolve("./dist")
const indexPath = resolve(distDir, "index.html")

const contentTypes = {
  html: "text/html; charset=utf-8",
  css: "text/css; charset=utf-8",
  js: "application/javascript; charset=utf-8",
  json: "application/json; charset=utf-8",
  svg: "image/svg+xml",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  ico: "image/x-icon",
  woff: "font/woff",
  woff2: "font/woff2",
  ttf: "font/ttf",
}

function responseFromFile(filePath, headers) {
  try {
    return new Response(readFileSync(filePath), { headers })
  } catch (error) {
    if (error?.code === "ENOENT") {
      return new Response("Not Found", { status: 404 })
    }

    console.error("File read error:", error)
    return new Response("Internal Server Error", { status: 500 })
  }
}

function serveIndex() {
  return responseFromFile(indexPath, {
    "Content-Type": contentTypes.html,
    "Cache-Control": "no-cache",
  })
}

function serveFile(filePath, pathname) {
  const ext = filePath.split(".").pop() || ""
  return responseFromFile(filePath, {
    "Content-Type": contentTypes[ext] || "application/octet-stream",
    "Cache-Control": pathname.startsWith("/assets/") ? "public, max-age=31536000, immutable" : "no-cache",
  })
}

Bun.serve({
  port: 80,
  hostname: "0.0.0.0",
  fetch(req) {
    const url = new URL(req.url)
    const pathname = url.pathname
    const filePath = resolve(distDir, pathname.slice(1))
    const insideDist = filePath === distDir || filePath.startsWith(`${distDir}${sep}`)

    if (!insideDist) {
      return new Response("Forbidden", { status: 403 })
    }

    if (pathname !== "/" && existsSync(filePath)) {
      return serveFile(filePath, pathname)
    }

    if (!pathname.includes(".") && existsSync(indexPath)) {
      return serveIndex()
    }

    return new Response("Not Found", { status: 404 })
  },
})
