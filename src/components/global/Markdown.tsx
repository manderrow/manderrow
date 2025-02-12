import { parse } from "marked";
import dompurify from "dompurify";
import { JSX } from "solid-js";

interface MarkdownComponentOptions {
  source: string;
  div?: JSX.HTMLAttributes<HTMLDivElement>;
}

export default function Markdown(options: MarkdownComponentOptions) {
  const escapedResult = dompurify.sanitize(parse(options.source, { async: false }));

  return <div innerHTML={escapedResult} {...options.div}></div>;
}
