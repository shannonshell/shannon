import { createServerFn } from "@tanstack/react-start";
import * as fs from "node:fs";
import * as path from "node:path";

const DOCS_DIR = path.resolve(process.cwd(), "../docs");

export const getDocPage = createServerFn({ method: "GET" })
  .inputValidator((slug: string) => slug)
  .handler(async ({ data: slug }) => {
    // Sanitize slug to prevent path traversal
    const safe = slug.replace(/\.\./g, "").replace(/^\//, "");
    const filePath = path.join(DOCS_DIR, `${safe}.md`);

    if (!fs.existsSync(filePath)) {
      throw new Error(`Doc not found: ${safe}`);
    }

    const content = fs.readFileSync(filePath, "utf-8");
    const titleMatch = content.match(/^#\s+(.+)$/m);
    const title = titleMatch ? titleMatch[1] : safe;

    return { slug: safe, title, content };
  });
