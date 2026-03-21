import { createFileRoute, Link } from "@tanstack/react-router";
import docsData from "../../../data/docs.json";

export const Route = createFileRoute("/docs/")({
  head: () => ({ meta: [{ title: "Docs — Shannon" }] }),
  component: DocsIndex,
});

interface DocPage {
  slug: string;
  title: string;
  section: string | null;
}

function DocsIndex() {
  const pages = docsData.pages as DocPage[];

  // Group by section
  const topLevel = pages.filter((p) => p.section === null);
  const sections = new Map<string, DocPage[]>();
  for (const page of pages) {
    if (page.section) {
      const list = sections.get(page.section) || [];
      list.push(page);
      sections.set(page.section, list);
    }
  }

  return (
    <section className="text-sm">
      <h1 className="text-lg font-bold text-foreground mb-6">Documentation</h1>

      {topLevel.length > 0 && (
        <ul className="mb-4 space-y-1">
          {topLevel.map((page) => (
            <li key={page.slug}>
              <Link
                to={`/docs/${page.slug}`}
                className="text-accent hover:text-primary"
              >
                {page.title}
              </Link>
            </li>
          ))}
        </ul>
      )}

      {Array.from(sections.entries()).map(([section, sectionPages]) => (
        <div key={section} className="mb-4">
          <h2 className="text-sm font-bold text-primary mb-2">
            ┌─ {section} ─┐
          </h2>
          <ul className="space-y-1 pl-2">
            {sectionPages.map((page) => (
              <li key={page.slug}>
                <Link
                  to={`/docs/${page.slug}`}
                  className="text-accent hover:text-primary"
                >
                  {page.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
