import { faHardDrive } from "@fortawesome/free-regular-svg-icons";
import Fa from "solid-fa";
import { Accessor, For } from "solid-js";

import { ModId } from "../../../api/api.ts";
import { t } from "../../../i18n/i18n.ts";
import { humanizeFileSize } from "../../../utils/utils.ts";
import { getIconUrl, getQualifiedModName } from "./common.ts";

import styles from "./BulkActions.module.css";

const MAX_SELECTED_CARDS = 5;

export default function BulkActions(props: { mods: Accessor<(ModId & { version: string })[]> }) {
  const selectedCount = () => props.mods().length;

  return (
    <>
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

      <h2 class={styles.selected__title}>{t("modlist.installed.multiselect_title")}</h2>

      <ul>
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
              .reduce((total, { name, owner, version }) => total /*+ getModById({name, owner}).file_size*/, 0),
          )}
        </li>
      </ul>

      <div class={styles.selected__actions}>
        <button>{t("modlist.installed.enable_selected")}</button>
        <button>{t("modlist.installed.disable_selected")}</button>
        <button>{t("global.phrases.delete")}</button>
        <button>{t("modlist.installed.update_selected")}</button>
      </div>
    </>
  );
}
