import { createStartHandler, defaultStreamHandler } from "@tanstack/react-start/server";
import { join } from "node:path";
import { existsSync, readFileSync } from "node:fs";

const handler = createStartHandler(defaultStreamHandler);

const CLIENT_DIR = join(import.meta.dirname, "..", "client");
const DOCS_DIR = join(import.meta.dirname, "..", "..", "..", "docs");

export default {
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    // Serve raw markdown for /docs/<slug>.md
    const mdMatch = url.pathname.match(/^\/docs\/([a-zA-Z0-9_/-]+)\.md$/);
    if (mdMatch) {
      const filePath = join(DOCS_DIR, `${mdMatch[1]}.md`);
      if (existsSync(filePath)) {
        const content = readFileSync(filePath, "utf-8");
        return new Response(content, {
          headers: { "Content-Type": "text/markdown; charset=utf-8" },
        });
      }
      return new Response("Not found", { status: 404 });
    }

    // Serve static assets from dist/client
    if (url.pathname.startsWith("/assets/") || url.pathname.startsWith("/_build/")) {
      const filePath = join(CLIENT_DIR, url.pathname);
      const file = Bun.file(filePath);
      if (await file.exists()) {
        return new Response(file);
      }
    }

    // Serve public files (images, favicon, etc.)
    const publicPath = join(CLIENT_DIR, url.pathname);
    if (url.pathname !== "/" && existsSync(publicPath)) {
      const file = Bun.file(publicPath);
      if (await file.exists()) {
        return new Response(file);
      }
    }

    // SSR for everything else
    return handler(request);
  },
};
