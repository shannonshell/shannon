import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <section className="text-sm">
      <div className="flex items-center gap-4 mb-6">
        <img
          src="/images/shannon-1.webp"
          alt="Shannon logo"
          className="w-16 h-16"
        />
        <div>
          <h1 className="text-lg font-bold text-foreground">Shannon</h1>
          <p className="text-muted">
            An AI-first shell with seamless access to bash, nushell, and any
            other shell.
          </p>
        </div>
      </div>

      <p className="text-foreground-dark mb-4">
        Type in plain English and have an LLM translate your intent into the
        right command. Press <span className="text-accent">Shift+Tab</span> to
        drop into bash, nushell, or any other shell — then Shift+Tab back.
      </p>

      <div className="mt-6">
        <Link
          to="/docs"
          className="text-primary hover:text-accent"
        >
          {">"} [Read the docs]
        </Link>
      </div>
    </section>
  );
}
