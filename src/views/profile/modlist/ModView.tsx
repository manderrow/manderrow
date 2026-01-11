import { faHeart } from "@fortawesome/free-regular-svg-icons";
import { faDownload, faExternalLink, faHardDrive, faXmark } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { createResource, createSelector, createSignal, For, Show, useContext } from "solid-js";

import { fetchModIndex, getFromModIndex } from "../../../api/api";
import { createProgressProxyStore, initProgress } from "../../../api/tasks";
import { t } from "../../../i18n/i18n";
import { Mod, ModListing, ModPackage, ModVersion } from "../../../types";
import { dateFormatterMed, humanizeFileSize, removeProperty, roundedNumberFormatter } from "../../../utils/utils";
import { getIconUrl, getModAuthorUrl, getModUrl, getModVersionUrl, useInstalled } from "./common";

import SelectDropdown from "../../../widgets/SelectDropdown";
import TabRenderer, { Tab, TabContent } from "../../../widgets/TabRenderer";
import TogglableDropdown from "../../../widgets/TogglableDropdown";
import { InstallButton, UninstallButton } from "./InstallationBtns";
import { ModInstallContext } from "./ModList";
import ModMarkdown from "./ModMarkdown";

import styles from "./ModView.module.css";

export default function ModView(props: { mod: Mod; gameId: string; closeModView: () => void }) {
  const [_progress, setProgress] = createProgressProxyStore();

  function getInitialModListing(mod: Mod) {
    if ("version" in mod) {
      const obj: ModListing & { game?: string; version?: ModVersion } = { ...mod, versions: [mod.version] };
      delete obj.version;
      delete obj.game;
      return obj;
    } else {
      return mod;
    }
  }

  const [modListing] = createResource<ModListing | undefined, Mod | Record<never, never>, never>(
    () => props.mod,
    async (mod) => {
      if ("version" in mod) {
        await fetchModIndex(props.gameId, { refresh: false }, (event) => {
          if (event.event === "created") {
            setProgress(event.progress);
          }
        });
        return (await getFromModIndex(props.gameId, [{ owner: mod.owner, name: mod.name }]))[0];
      } else if ("versions" in mod) {
        setProgress(initProgress());
        return mod;
      }
    },
    { initialValue: getInitialModListing(props.mod) },
  );

  const [selectedVersion, setSelectedVersion] = createSignal<string>();

  const modVersionData = () => {
    const selected = selectedVersion();

    const versions = "versions" in props.mod ? props.mod.versions : modListing?.latest?.versions;

    // If online, display the mod listing. Otherwise, in installed, display mod package initially, then
    // mod listing after it loads. Coincidentally, this undefined logic check works as the mod listing
    // loads after the initial mod package is always displayed by default first
    return (
      (selected === undefined ? undefined : versions?.find((v) => v.version_number === selected)) ??
      ("versions" in props.mod ? props.mod.versions[0] : props.mod.version)
    );
  };

  const tabs: Tab<"overview" | "dependencies" | "changelog">[] = [
    {
      id: "overview",
      name: "Overview",
      component: () => <ModMarkdown mod={props.mod} selectedVersion={selectedVersion()} endpoint="readme" />,
    },
    {
      id: "dependencies",
      name: "Dependencies",
      component: () => <ModViewDependencies dependencies={modVersionData().dependencies} gameId={props.gameId} />,
    },
    {
      id: "changelog",
      name: "Changelog",
      component: () => <ModMarkdown mod={props.mod} selectedVersion={selectedVersion()} endpoint="changelog" />,
    },
  ];

  const [currentTab, setCurrentTab] = createSignal(tabs[0].id);
  const isCurrentTab = createSelector(currentTab);

  const installContext = useContext(ModInstallContext);

  const isSelectedVersion = createSelector(selectedVersion);

  const installed = useInstalled(installContext, () => props.mod);

  return (
    <div class={styles.modView}>
      <div class={styles.modSticky}>
        <div class={styles.modMeta}>
          {/* TODO: For local mod with no package URL, remove link */}
          <div style={{ "grid-area": "name" }}>
            <a
              href={getModUrl(props.gameId, props.mod.owner, props.mod.name)}
              target="_blank"
              rel="noopener noreferrer"
              class={styles.modMetaLink}
            >
              <h2 class={styles.name}>{props.mod.name}</h2>
              <Fa icon={faExternalLink} />
            </a>
          </div>
          <div style={{ "grid-area": "owner" }}>
            <a
              href={getModAuthorUrl(props.gameId, props.mod.owner)}
              target="_blank"
              rel="noopener noreferrer"
              class={styles.modMetaLink}
            >
              {props.mod.owner}
              <Fa icon={faExternalLink} />
            </a>
          </div>
          <ul class={styles.modMetadata}>
            <li class={styles.metadata__field}>v{modVersionData().version_number}</li>
            <li class={styles.metadata__field}>
              <Fa icon={faDownload} /> {roundedNumberFormatter.format(modVersionData().downloads)}
            </li>
            <li class={styles.metadata__field}>
              <Fa icon={faHardDrive} /> {humanizeFileSize(modVersionData().file_size)}
            </li>
          </ul>

          <Show when={props.mod.donation_link != null}>
            <a class={styles.modMeta__donate} href={props.mod.donation_link} target="_blank" rel="noopener noreferrer">
              <Fa icon={faHeart} class={styles.donate__icon} />
              <br /> {t("modlist.modview.donate_btn")}
            </a>
          </Show>

          <button style={{ "grid-area": "close" }} class={styles.modMeta__closeBtn} onClick={props.closeModView}>
            <Fa icon={faXmark} />
          </button>
        </div>

        <TabRenderer
          id="mod-view"
          tabs={tabs}
          styles={{
            preset: "base",
            classes: {
              container: styles.tabs,
              tab: styles.tabs__tab,
            },
          }}
          setter={(tab) => setCurrentTab(tab.id)}
        />
      </div>

      <div class={styles.modView__content}>
        <TabContent isCurrentTab={isCurrentTab} tabs={tabs} />
      </div>

      <form class={styles.modView__form} action="#">
        <Show
          when={installed()}
          fallback={
            <div class={styles.modView__onlineActions}>
              <SelectDropdown<string>
                options={
                  modListing.latest?.versions.map((version, i) => ({
                    label: version.version_number,
                    value: version.version_number,
                    selected: () =>
                      selectedVersion() == null && i === 0 ? true : isSelectedVersion(version.version_number),
                    liContent: (
                      <div>
                        <p data-version>{version.version_number}</p>
                        <p data-date>{dateFormatterMed.format(new Date(version.date_created))}</p>
                      </div>
                    ),
                  })) ?? []
                }
                label={{ labelText: "value" }}
                labelClass={styles.modView__versions}
                onChanged={(value) => setSelectedVersion(value)}
                liClass={styles.modView__versionsItem}
              />
              <InstallButton
                mod={props.mod as ModListing}
                installContext={installContext!}
                class={styles.modView__downloadBtn}
              >
                {t("modlist.online.install_btn")}
              </InstallButton>
            </div>
          }
        >
          <div class={styles.modView__installedActions}>
            <TogglableDropdown
              label={t("modlist.installed.change_version_btn")}
              labelClass={styles.modView__versionLabel}
              dropdownClass={styles.modView__versionsDropdownContent}
              fillToTriggerWidth
            >
              <input
                type="text"
                name="version-search"
                id="version-search"
                placeholder={t("modlist.installed.search_version_placeholder")}
              />
              <label for="version-search" class="phantom">
                {t("modlist.installed.search_version_placeholder")}
              </label>

              <Show when={modListing.latest}>
                {(listing) => (
                  <>
                    <SelectDropdown
                      label={{ labelText: "value" }}
                      onChanged={setSelectedVersion}
                      options={(listing().versions ?? []).map((version) => ({
                        label: version.version_number,
                        value: version.version_number,
                        selected: () => isSelectedVersion(version.version_number),
                      }))}
                    />

                    <InstallButton
                      mod={
                        {
                          ...removeProperty(listing(), "versions"),
                          version:
                            listing().versions.find((v) => v.version_number === selectedVersion()) ??
                            listing().versions[0],
                        } as ModPackage
                      }
                      installContext={installContext!}
                      class={styles.downloadBtn}
                    >
                      {t("global.phrases.apply")}
                    </InstallButton>
                  </>
                )}
              </Show>
            </TogglableDropdown>
            <UninstallButton mod={installed()!} installContext={installContext!} class={styles.modView__uninstallBtn}>
              {t("modlist.installed.uninstall_btn")}
            </UninstallButton>
          </div>
        </Show>
      </form>
    </div>
  );
}

function ModViewDependencies(props: { gameId: string; dependencies: string[] }) {
  return (
    <Show when={props.dependencies.length > 0} fallback={<p>{t("modlist.modview.no_dependencies_msg")}</p>}>
      <ul class={styles.modDeps}>
        <For each={props.dependencies}>
          {(dependency) => {
            const [author, name, version] = dependency.split("-");

            return (
              <li>
                <a
                  class={styles.dependency}
                  href={getModVersionUrl(props.gameId, author, name, version)}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <img src={getIconUrl(dependency)} width="48px" alt={name} class={styles.modIcon} />
                  <div>
                    <p data-name>
                      {name} <Fa icon={faExternalLink} class={styles.externalIcon} />
                    </p>
                    <p data-owner>{author}</p>
                  </div>
                  <p data-version>{version}</p>
                </a>
              </li>
            );
          }}
        </For>
      </ul>
    </Show>
  );
}
