import { createServerFn } from "@tanstack/react-start";
import * as fs from "node:fs";
import * as path from "node:path";

const DOCS_DIR = path.resolve(process.cwd(), "../docs");

// Resolve a slug to a file path, handling number-prefixed filenames.
// e.g. "getting-started" finds "docs/01-getting-started.md"
// e.g. "features/shell-switching" finds "docs/features/01-shell-switching.md"
function resolveDocPath(slug: string): string | null {
  const parts = slug.split("/");
  let dir = DOCS_DIR;

  // Resolve directory parts (may have number prefixes)
  for (let i = 0; i < parts.length - 1; i++) {
    const found = findNumbered(dir, parts[i]);
    if (!found) return null;
    dir = path.join(dir, found);
  }

  // Resolve the file part
  const basename = parts[parts.length - 1];
  const found = findNumbered(dir, basename, ".md");
  if (!found) return null;
  return path.join(dir, found);
}

// Find an entry in a directory that matches `name` with an optional number prefix.
// e.g. findNumbered("/docs", "features") matches "features" or "01-features"
function findNumbered(dir: string, name: string, suffix: string = ""): string | null {
  const exact = name + suffix;
  if (fs.existsSync(path.join(dir, exact))) return exact;

  try {
    for (const entry of fs.readdirSync(dir)) {
      const stripped = entry.replace(/^\d+-/, "");
      if (stripped === exact) return entry;
    }
  } catch {
    // Directory doesn't exist
  }
  return null;
}

export const getDocPage = createServerFn({ method: "GET" })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    const safe = slug.replace(/\.\./g, "").replace(/^\//, "");
    const filePath = resolveDocPath(safe);

    if (!filePath) {
      throw new Error(`Doc not found: ${safe}`);
    }

    const content = fs.readFileSync(filePath, "utf-8");
    const titleMatch = content.match(/^#\s+(.+)$/m);
    const title = titleMatch ? titleMatch[1] : safe;

    return { slug: safe, title, content };
  });
