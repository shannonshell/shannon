import { defineConfig, type Plugin } from "vite";
import tsConfigPaths from "vite-tsconfig-paths";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import * as fs from "node:fs";
import * as path from "node:path";

function docsMarkdownPlugin(): Plugin {
  const docsDir = path.resolve(__dirname, "../docs");

  return {
    name: "docs-markdown",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const match = req.url?.match(/^\/docs\/([a-zA-Z0-9_/-]+)\.md$/);
        if (!match) return next();

        const filePath = path.join(docsDir, `${match[1]}.md`);
        if (!fs.existsSync(filePath)) {
          res.statusCode = 404;
          res.end("Not found");
          return;
        }

        const content = fs.readFileSync(filePath, "utf-8");
        res.setHeader("Content-Type", "text/markdown; charset=utf-8");
        res.end(content);
      });
    },
  };
}

export default defineConfig({
  server: {
    port: 3000,
  },
  plugins: [docsMarkdownPlugin(), tsConfigPaths(), tanstackStart(), tailwindcss()],
});
