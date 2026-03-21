import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/docs/")({
  head: () => ({ meta: [{ title: "Docs — Shannon" }] }),
  component: DocsIndex,
});

function DocsIndex() {
  return (
    <section className="text-sm">
      <h1 className="text-lg font-bold text-foreground mb-4">Documentation</h1>
      <p className="text-foreground-dark">
        Shannon is an AI-first shell with seamless access to bash, nushell, and
        any other shell. Select a topic from the sidebar to get started.
      </p>
    </section>
  );
}
