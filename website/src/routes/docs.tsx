import { createFileRoute, Outlet } from "@tanstack/react-router";
import { DocsSidebar } from "../components/DocsSidebar";

export const Route = createFileRoute("/docs")({
  component: DocsLayout,
});

function DocsLayout() {
  return (
    <div className="flex gap-6">
      <DocsSidebar />
      <div className="flex-1 min-w-0">
        <Outlet />
      </div>
    </div>
  );
}
