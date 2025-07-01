import { parse, marked, Renderer, Parser } from "marked";
import dompurify from "dompurify";
import { JSX } from "solid-js";
import markedAlert from "marked-alert";
import * as emoji from "node-emoji";

function replaceEmojis(markdown: string) {
  return marked(
    markdown.replace(/(:.*:)/g, (match) => emoji.emojify(match)),
    { async: false },
  );
}

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
  const escapedResult = () => dompurify.sanitize(parse(replaceEmojis(options.source), { async: false }));

  return <div innerHTML={escapedResult()} {...options.div}></div>;
}
