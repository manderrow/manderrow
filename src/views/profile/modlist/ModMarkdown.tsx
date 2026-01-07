import { Show, createResource } from "solid-js";
import { Endpoint, fetchModMarkdown } from "../../../api/mod_index/thunderstore";
import { Mod } from "../../../types";
import Markdown from "../../../widgets/Markdown";
import { createProgressProxyStore } from "../../../api/tasks";
import { SimpleProgressIndicator } from "../../../widgets/Progress";
import { t } from "../../../i18n/i18n";

export default function ModMarkdown(props: {
  mod: Mod | undefined;
  selectedVersion: string | undefined;
  endpoint: Endpoint;
}) {
  const modData: () => { mod?: never; version?: never } | { mod: Mod; version: string } = () => {
    const m = props.mod;
    // mod is a ModPackage if it has the version field, otherwise it is a ModListing
    if (m == undefined) return {};
    return {
      mod: m,
      version: props.selectedVersion ?? ("version" in m ? m.version.version_number : m.versions[0].version_number),
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
        <Show when={resource().markdown} fallback={<p>{t(`profile.mod_markdown.no_${props.endpoint}_provided`)}</p>}>
          {(markdown) => <Markdown source={markdown()} div={{ class: "markdown" }} />}
        </Show>
      )}
    </Show>
  );
}
