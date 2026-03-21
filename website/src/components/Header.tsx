import { Link, useRouterState } from "@tanstack/react-router";

const NAV_ITEMS = [
  { to: "/docs", label: "Docs" },
] as const;

export function Header() {
  const { location } = useRouterState();

  return (
    <header className="mb-8">
      <div className="flex items-center justify-between text-sm">
        <Link to="/" className="flex items-center gap-2 text-primary font-bold">
          <img
            src="/images/shannon-1.webp"
            alt="Shannon logo"
            className="w-6 h-6"
          />
          Shannon
        </Link>
        <nav className="flex gap-1">
          {NAV_ITEMS.map(({ to, label }) => {
            const active = location.pathname.startsWith(to);
            return (
              <Link
                key={to}
                to={to}
                className={active ? "text-primary" : "text-muted hover:text-accent"}
              >
                {active ? `>[${label}]` : `[${label}]`}
              </Link>
            );
          })}
        </nav>
      </div>
      <div className="mt-3 text-muted text-xs">
        ────────────────────────────────────────────────────────────────────
      </div>
    </header>
  );
}
