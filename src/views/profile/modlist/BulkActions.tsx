import { faHardDrive } from "@fortawesome/free-regular-svg-icons";
import Fa from "solid-fa";
import { Accessor, createSignal, For, useContext } from "solid-js";

import { ModId } from "../../../api/api.ts";
import { t } from "../../../i18n/i18n.ts";
import { humanizeFileSize } from "../../../utils/utils.ts";
import { concatenateMod, getIconUrl, getQualifiedModName } from "./common.ts";
import { bindValue } from "../../../components/Directives.tsx";

import styles from "./BulkActions.module.css";
import Tooltip, { TooltipTrigger } from "../../../widgets/Tooltip.tsx";
import { faXmark } from "@fortawesome/free-solid-svg-icons";
import { ModInstallContext } from "./ModList.tsx";
import { ModPackage } from "../../../types";

const MAX_SELECTED_CARDS = 5;

interface BulkActionsProps {
  mods: Accessor<(ModId & { version: string })[]>;
  deleteSelectedMod: (mod: ModPackage) => void;
  clearSelection: () => void;
}

export default function BulkActions(props: BulkActionsProps) {
  const selectedCount = () => props.mods().length;

  const installContext = useContext(ModInstallContext)!;

  function getInstalledMod(modId: ModId) {
    return installContext.installed().find((mod) => mod.name === modId.name && mod.owner === modId.owner);
  }

  const [search, setSearch] = createSignal("");

  return (
    <div class={styles.selected}>
      <div class={styles.selected__cards} aria-hidden>
        <ul
          class={styles.cards__list}
          style={{
            "--total-cards": Math.min(selectedCount(), MAX_SELECTED_CARDS),
          }}
        >
          <For each={props.mods().slice(0, MAX_SELECTED_CARDS)}>
            {({ name, owner, version }, i) => {
              return (
                <li
                  class={styles.selected__card}
                  style={{
                    "--card-index": i(),
                    "--bg-image": `url("${getIconUrl(getQualifiedModName(owner, name, version))}")`,
                  }}
                  data-overflow={
                    i() === MAX_SELECTED_CARDS - 1 ? `+${selectedCount() - MAX_SELECTED_CARDS + 1}` : undefined
                  }
                ></li>
              );
            }}
          </For>
        </ul>
      </div>

      <header class={styles.selected__header}>
        <h2 class={styles.selected__title}>{t("modlist.installed.multiselect_title")}</h2>

        <label for="bulk-search" class="phantom">
          Search selected mods
        </label>
        <input
          type="text"
          name="bulk-search"
          id="bulk-search"
          placeholder={t("global.phrases.search")}
          use:bindValue={[search, setSearch]}
        />
      </header>

      <div class={styles.selected__tableContainer}>
        <ul class={styles.selected__table}>
          <For
            each={props
              .mods()
              .filter(({ name, owner, version }) =>
                concatenateMod(owner, name, version).toLowerCase().includes(search().toLowerCase()),
              )}
          >
            {({ name, owner, version }) => {
              const mod = getInstalledMod({ name, owner });
              if (mod == null)
                throw new Error(
                  "Selected mod not installed, this should be impossible. Is the bulk actions within the InstallContext?",
                );

              return (
                <li>
                  <span>
                    <Tooltip content={t("global.phrases.remove")}>
                      <TooltipTrigger class={styles.removeBtn} onClick={() => props.deleteSelectedMod(mod)}>
                        <Fa icon={faXmark} />
                      </TooltipTrigger>
                    </Tooltip>
                  </span>
                  <p class={styles.mod__info}>
                    <span>{mod.name}</span>
                    <span class={styles.separator}>•</span>
                    <span class={styles.lowHierarchy}>{mod.owner}</span>
                    <span class={styles.separator}>•</span>
                    <span class={styles.lowHierarchy}>{mod.version.version_number}</span>
                  </p>
                  <span>{humanizeFileSize(mod.version.file_size)}</span>
                </li>
              );
            }}
          </For>
        </ul>
      </div>

      <ul class={styles.summary__row}>
        <li>
          {t(
            selectedCount() > 1 ? "modlist.installed.selected_count_plural" : "modlist.installed.selected_count_single",
            { count: selectedCount() },
          )}
        </li>
        <li>
          <Fa icon={faHardDrive} />{" "}
          {humanizeFileSize(
            props
              .mods()
              .reduce(
                (total, { name, owner, version }) =>
                  total + (getInstalledMod({ name, owner })!.version!.file_size ?? 0),
                0,
              ),
          )}
        </li>
      </ul>

      <div class={styles.selected__actions}>
        <button>{t("modlist.installed.enable_all")}</button>
        <button>{t("modlist.installed.disable_all")}</button>
        <button>{t("modlist.installed.delete_all")}</button>
        <button>{t("modlist.installed.update_all")}</button>
        <button onClick={props.clearSelection}>{t("global.phrases.cancel")}</button>
      </div>
    </div>
  );
}
