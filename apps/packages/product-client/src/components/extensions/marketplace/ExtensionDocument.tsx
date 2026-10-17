import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useExtensionDocument } from "#product/hooks/access/extensions/use-extension-registry";
import { resolveExtensionContentUrl } from "#product/lib/domain/extensions/extension-content";
import { EXTENSION_MARKETPLACE_COPY as copy } from "#product/copy/extensions/marketplace";
import { ExtensionRegistryFailure } from "./ExtensionRegistryFailure";

export function ExtensionDocument({ url }: { url: string | null }) {
  const document = useExtensionDocument(url);
  if (!url) return <p className="p-5 text-ui text-muted-foreground">{copy.noDocument}</p>;
  if (document.isError) return <ExtensionRegistryFailure error={document.error} onRetry={() => { void document.refetch(); }} />;
  if (document.isPending) return <p role="status" className="p-5 text-ui text-muted-foreground">Loading document…</p>;
  return (
    <div className="min-w-0 break-words p-5 text-ui leading-relaxed text-foreground [&_h1]:mb-4 [&_h1]:text-xl [&_h1]:font-semibold [&_h2]:my-4 [&_h2]:text-lg [&_h2]:font-semibold [&_p]:my-3 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5 [&_pre]:overflow-auto [&_pre]:rounded-md [&_pre]:bg-surface-control [&_pre]:p-3 [&_code]:font-mono [&_table]:block [&_table]:overflow-auto [&_td]:border [&_td]:border-border [&_td]:p-2 [&_th]:p-2">
      <ReactMarkdown remarkPlugins={[remarkGfm]} skipHtml
        urlTransform={(value) => resolveExtensionContentUrl(value, url) ?? ""}
        components={{
          a: ({ href, children }) => href
            ? <a href={href} target="_blank" rel="noopener noreferrer" className="text-primary underline underline-offset-2">{children}</a>
            : <span>{children}</span>,
          img: ({ src, alt }) => src
            ? <img src={src} alt={alt ?? ""} loading="lazy" referrerPolicy="no-referrer" className="max-w-full rounded-md" />
            : null,
        }}>
        {document.data}
      </ReactMarkdown>
    </div>
  );
}
