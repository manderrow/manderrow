import { Show, createResource } from "solid-js";
import { fetchModMarkdown } from "../../api/mod_index/thunderstore";
import { Mod } from "../../types";
import Markdown from "../global/Markdown";
import { createProgressProxyStore } from "../../api/tasks";
import { SimpleProgressIndicator } from "../global/Progress";

const LABELS = Object.freeze({
  readme: "README",
  changelog: "changelog",
});

export default function ModMarkdown(props: {
  mod: Mod | undefined;
  selectedVersion: string | undefined;
  endpoint: "readme" | "changelog";
}) {
  const modData: () => { mod?: never; version?: never } | { mod: Mod; version: string } = () => {
    const m = props.mod;
    // mod is a ModPackage if it has the version field, otherwise it is a ModListing
    if (m == undefined) return {};
    return {
      mod: m,
      version: "version" in m ? m.version.version_number : props.selectedVersion ?? m.versions[0].version_number,
    };
  };

  const [progress, setProgress] = createProgressProxyStore();

  const [resource] = createResource(
    modData,
    async ({ mod, version }: { mod?: never; version?: never } | { mod: Mod; version: string }) => {
      if (mod == null) return undefined;

      return await fetchModMarkdown(mod.owner, mod.name, version, props.endpoint, (event) => {
        if (event.event === "created") {
          setProgress!(event.progress);
        }
      });
    },
  );

  return (
    <Show
      when={!resource.loading && resource.state !== "errored" ? resource() : undefined}
      fallback={
        <Show when={resource.error} fallback={<SimpleProgressIndicator progress={progress} />}>
          {(error) => <p>{error().toString()}</p>}
        </Show>
      }
    >
      {(resource) => (
        <Show when={resource().markdown} fallback={<p>No {LABELS[props.endpoint]} provided.</p>}>
          {(markdown) => <Markdown source={markdown()} div={{ class: "markdown" }} />}
        </Show>
      )}
    </Show>
  );
}
