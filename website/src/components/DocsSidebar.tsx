import { Link, useRouterState } from "@tanstack/react-router";
import docsData from "../../data/docs.json";

interface DocPage {
  slug: string;
  title: string;
  section: string | null;
}

export function DocsSidebar() {
  const { location } = useRouterState();
  const pages = docsData.pages as DocPage[];

  const topLevel = pages.filter((p) => p.section === null);
  const sections = new Map<string, DocPage[]>();
  for (const page of pages) {
    if (page.section) {
      const list = sections.get(page.section) || [];
      list.push(page);
      sections.set(page.section, list);
    }
  }

  function isActive(slug: string) {
    return location.pathname === `/docs/${slug}`;
  }

  function linkClass(slug: string) {
    return isActive(slug)
      ? "text-primary font-bold"
      : "text-accent hover:text-primary";
  }

  return (
    <nav className="text-xs space-y-3 pr-4 border-r border-border min-w-[180px]">
      <ul className="space-y-1">
        {topLevel.map((page) => (
          <li key={page.slug}>
            <Link to={`/docs/${page.slug}`} className={linkClass(page.slug)}>
              {isActive(page.slug) ? `> ${page.title}` : page.title}
            </Link>
          </li>
        ))}
      </ul>

      {Array.from(sections.entries()).map(([section, sectionPages]) => (
        <div key={section}>
          <div className="text-muted mb-1">┌─ {section} ─┐</div>
          <ul className="space-y-1 pl-2">
            {sectionPages.map((page) => (
              <li key={page.slug}>
                <Link to={`/docs/${page.slug}`} className={linkClass(page.slug)}>
                  {isActive(page.slug) ? `> ${page.title}` : page.title}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
}
