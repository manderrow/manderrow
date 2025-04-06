import { parse, marked, Renderer, Parser } from "marked";
import dompurify from "dompurify";
import { JSX } from "solid-js";
import markedAlert from "marked-alert";

marked.use(markedAlert());
marked.use({
  renderer: {
    table(tokens) {
      const renderer = new Renderer();
      renderer.parser = new Parser();
      return `<div class="table-wrapper">${renderer.table(tokens)}</div>`;
    },
  },
});

interface MarkdownComponentOptions {
  source: string;
  div?: JSX.HTMLAttributes<HTMLDivElement>;
}

export default function Markdown(options: MarkdownComponentOptions) {
  const escapedResult = () => dompurify.sanitize(parse(options.source, { async: false }));

  return <div innerHTML={escapedResult()} {...options.div}></div>;
}
