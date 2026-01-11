import { faHardDrive } from "@fortawesome/free-regular-svg-icons";
import { faDownload, faDownLong, faTrash } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { createMemo, Match, Show, Switch, useContext } from "solid-js";

import { ModId } from "../../../api/api";
import { t } from "../../../i18n/i18n";
import { Mod, ModListing, ModPackage } from "../../../types";
import { humanizeFileSize, roundedNumberFormatter } from "../../../utils/utils";
import { ModInstallContext, SelectableModListProps } from "./ModList";
import { getIconUrl, getQualifiedModName, useInstalled } from "./common";

import ErrorBoundary from "../../../components/ErrorBoundary";
import Checkbox from "../../../widgets/Checkbox";
import Tooltip, { TooltipTrigger } from "../../../widgets/Tooltip";
import { InstallButton, UninstallButton } from "./InstallationBtns";

import styles from "./ModListItem.module.css";

export default function ModListItem(
  props: {
    mod: Mod;
    /// Whether the mod is focused in the ModView
    isFocused: (mod: ModId) => boolean;
    setFocused: (mod: ModId | undefined) => void;
    forceSelectorVisibility: boolean;
    select?: () => void;
    shiftClick?: () => void;
    isSelected?: () => boolean;
    isPivot?: boolean;
    // setModSelectorTutorialState: (hovered: boolean) => void,
  } & (Omit<Partial<SelectableModListProps>, "select" | "shiftClick" | "isPivot"> | {}),
) {
  const displayVersion = createMemo(() => {
    if ("version" in props.mod) return props.mod.version;
    return props.mod.versions[0];
  });

  const installContext = useContext(ModInstallContext);
  const installed = useInstalled(installContext, () => props.mod);

  function onFocus() {
    const isFocused = props.isFocused(props.mod);

    props.setFocused(
      isFocused
        ? undefined
        : {
            owner: props.mod.owner,
            name: props.mod.name,
          },
    );
  }

  return (
    <li
      classList={{
        [styles.mod]: true,
        [styles.selected]: props.isFocused(props.mod),
      }}
    >
      <div
        onClick={(e) => {
          if (e.shiftKey && "shiftClick" in props) {
            props.shiftClick!();

            // Prevent bubbling down to the onChange of
            // the in case the checkbox was clicked
            e.preventDefault();
          } else {
            onFocus();
          }
        }}
        onKeyDown={(key) => {
          if (key.key === "Enter") onFocus();
        }}
        class={styles.mod__btn}
        role="button"
        aria-pressed={props.isFocused(props.mod)}
        tabIndex={0}
      >
        <Show when={props.isSelected !== undefined}>
          <div class={styles.mod__selector} data-always-show={props.forceSelectorVisibility ? "" : undefined}>
            <Checkbox
              checked={props.isSelected!()}
              onChange={(checked) => props.select!()}
              labelClass={styles.mod__selectorClickRegion}
              iconContainerClass={styles.mod__selectorIndicator}
            />
          </div>
        </Show>
        <div class={styles.mod__btnContent}>
          <img
            class={styles.modIcon}
            width={64}
            alt="mod icon"
            src={getIconUrl(getQualifiedModName(props.mod.owner, props.mod.name, displayVersion().version_number))}
          />
          <div class={styles.mod__content}>
            <div class={styles.left}>
              <p class={styles.info}>
                <span class={styles.name}>{props.mod.name}</span>
                <span class={styles.separator} aria-hidden>
                  &bull;
                </span>
                <span class={styles.medHierarchy}>{props.mod.owner}</span>
                <Show when={"version" in props.mod}>
                  <span class={styles.separator} aria-hidden>
                    &bull;
                  </span>
                  <span class={styles.version}>{(props.mod as ModPackage).version.version_number}</span>
                </Show>
              </p>
              <p class={styles.info}>
                <Switch>
                  <Match when={"version" in props.mod}>
                    <span class={styles.lowHierarchy}>
                      <Fa icon={faHardDrive} /> {humanizeFileSize((props.mod as ModPackage).version.file_size)}
                    </span>
                  </Match>
                  <Match when={"versions" in props.mod}>
                    <span class={styles.lowHierarchy}>
                      <Fa icon={faDownload} />{" "}
                      {roundedNumberFormatter.format(
                        (props.mod as ModListing).versions.map((v) => v.downloads).reduce((acc, x) => acc + x),
                      )}
                    </span>
                  </Match>
                </Switch>
              </p>
              <p class={styles.description}>{displayVersion().description}</p>
            </div>
            <Show when={installContext !== undefined}>
              <Switch
                fallback={
                  <ErrorBoundary>
                    <Tooltip content={t("modlist.online.install_btn")}>
                      <TooltipTrigger
                        as={InstallButton}
                        mod={props.mod as ModListing}
                        installContext={installContext!}
                        class={styles.downloadBtn}
                        busyClass={styles.downloadBtnBusy}
                        progressStyle="circular"
                      >
                        <Fa icon={faDownLong} />
                      </TooltipTrigger>
                    </Tooltip>
                  </ErrorBoundary>
                }
              >
                <Match when={installed()}>
                  {(installed) => (
                    <ErrorBoundary>
                      <Tooltip content={t("modlist.installed.uninstall_btn")}>
                        <TooltipTrigger
                          as={UninstallButton}
                          mod={installed()}
                          installContext={installContext!}
                          class={styles.downloadBtn}
                          busyClass={styles.downloadBtnBusy}
                          progressStyle="circular"
                        >
                          <Fa icon={faTrash} />
                        </TooltipTrigger>
                      </Tooltip>
                    </ErrorBoundary>
                  )}
                </Match>
              </Switch>
            </Show>
          </div>
        </div>
      </div>
    </li>
  );
}
