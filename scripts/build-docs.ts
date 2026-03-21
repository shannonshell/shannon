import { readFileSync, readdirSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { join, relative, dirname } from "node:path";

const DOCS_DIR = join(import.meta.dirname, "..", "docs");
const OUT_FILE = join(import.meta.dirname, "..", "website", "data", "docs.json");

interface DocPage {
  slug: string;
  title: string;
  section: string | null;
}

function extractTitle(content: string): string {
  const match = content.match(/^#\s+(.+)$/m);
  return match ? match[1] : "Untitled";
}

function sectionName(slug: string): string | null {
  const parts = slug.split("/");
  if (parts.length < 2) return null;
  // Capitalize first letter of directory name
  const dir = parts[0];
  return dir.charAt(0).toUpperCase() + dir.slice(1);
}

function collectDocs(dir: string, base: string = ""): DocPage[] {
  const pages: DocPage[] = [];

  for (const entry of readdirSync(dir).sort()) {
    const fullPath = join(dir, entry);
    const stat = statSync(fullPath);

    if (stat.isDirectory()) {
      pages.push(...collectDocs(fullPath, base ? `${base}/${entry}` : entry));
    } else if (entry.endsWith(".md") && entry !== "README.md") {
      const slug = base ? `${base}/${entry.replace(/\.md$/, "")}` : entry.replace(/\.md$/, "");
      const content = readFileSync(fullPath, "utf-8");
      pages.push({
        slug,
        title: extractTitle(content),
        section: sectionName(slug),
      });
    }
  }

  return pages;
}

const pages = collectDocs(DOCS_DIR);

mkdirSync(dirname(OUT_FILE), { recursive: true });
writeFileSync(OUT_FILE, JSON.stringify({ pages }, null, 2) + "\n");
console.log(`Generated ${OUT_FILE} with ${pages.length} pages`);
