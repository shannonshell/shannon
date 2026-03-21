import { createFileRoute } from "@tanstack/react-router";
import { Markdown } from "../../components/Markdown";
import { getDocPage } from "../../server/docs";

export const Route = createFileRoute("/docs/$")({
  loader: async ({ params }) => {
    return getDocPage({ data: params._splat! });
  },
  head: ({ loaderData }) => ({
    meta: [{ title: `${loaderData?.title ?? "Docs"} — Shannon` }],
  }),
  component: DocPage,
});

function DocPage() {
  const { content } = Route.useLoaderData();

  return (
    <article className="prose-shannon">
      <Markdown content={content} />
    </article>
  );
}
