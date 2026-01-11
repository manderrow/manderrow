import { faArrowRightLong } from "@fortawesome/free-solid-svg-icons";
import Fa from "solid-fa";
import { createSignal, FlowProps } from "solid-js";

import { createProgressProxyStore } from "../../../api/tasks";
import { t } from "../../../i18n/i18n";
import { ModListing } from "../../../types";
import { getIconUrl, getQualifiedModName } from "./common";

import { DefaultDialog, DialogClose } from "../../../widgets/Dialog";

import styles from "./Updater.module.css";

export interface ModUpdate {
  newMod: ModListing;
  oldVersionNumber: string;
}

export default function ModUpdateDialogue(props: FlowProps & { updates: ModUpdate[] }) {
  const [_progress, _setProgress] = createProgressProxyStore();

  const [selectedMods, setSelectedMods] = createSignal<Set<ModUpdate>>(new Set(props.updates), {
    equals: false,
  });

  return (
    <DefaultDialog class={styles.updateDialog} trigger={props.children}>
      <h2>{t("modlist.installed.updater_title")}</h2>

      <div class={styles.listContainer}>
        <form action="#">
          <fieldset>
            <input
              type="checkbox"
              id="update-select-all-mods"
              checked={selectedMods().size === props.updates.length}
              onInput={(e) => {
                if (e.target.checked) {
                  setSelectedMods(new Set(props.updates));
                } else {
                  setSelectedMods(new Set<ModUpdate>());
                }
              }}
            />
            <label for="update-select-all-mods">{t("global.phrases.select_all")}</label>
          </fieldset>
          <fieldset>
            <label for="update-search" class="phantom">
              {t("global.phrases.search")}
            </label>
            <input type="text" id="update-search" placeholder={t("global.phrases.search")} />
          </fieldset>
        </form>
        <ul>
          {props.updates.map((update) => (
            <li>
              <label for={update.newMod.name}>
                <input
                  id={update.newMod.name}
                  type="checkbox"
                  checked={selectedMods().has(update)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      setSelectedMods((selectedMods) => selectedMods.add(update));
                    } else {
                      setSelectedMods((selectedMods) => {
                        selectedMods.delete(update);
                        return selectedMods;
                      });
                    }
                  }}
                />
                <img
                  width={48}
                  height={48}
                  alt="mod icon"
                  src={getIconUrl(
                    getQualifiedModName(
                      update.newMod.owner,
                      update.newMod.name,
                      update.newMod.versions[0].version_number,
                    ),
                  )}
                />
                <div class={styles.updateMetadata}>
                  <p data-name>{update.newMod.name}</p>
                  <p data-owner>{update.newMod.owner}</p>
                  <p data-version>
                    <span data-old-version>{update.oldVersionNumber}</span>
                    <span data-arrow>
                      <Fa icon={faArrowRightLong} />
                    </span>
                    <span data-new-version>{update.newMod.versions[0].version_number}</span>
                  </p>
                </div>
              </label>
            </li>
          ))}
        </ul>
      </div>

      <div class={styles.updateBtns}>
        <button data-btn="primary">
          {selectedMods().size === props.updates.length
            ? t("modlist.installed.update_all_btn")
            : t("modlist.installed.update_selected_btn")}
        </button>
        <DialogClose style={{ order: -1 }} data-btn="ghost">
          {t("global.phrases.cancel")}
        </DialogClose>
      </div>
    </DefaultDialog>
  );
}
