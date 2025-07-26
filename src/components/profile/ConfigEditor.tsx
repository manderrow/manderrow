import { useSearchParams } from "@solidjs/router";
import { t } from "../../i18n/i18n";

import styles from "./ConfigEditor.module.css";

export default function ConfigEditor() {
  const [searchParams, setSearchParams] = useSearchParams();

  return (
    <div class={styles.container}>
      <aside class={styles.configs}>
        <h1>{t("config.title")}</h1>
        <ul></ul>
      </aside>

      <div class={styles.editor}></div>

      <aside class={styles.sectionsOverview}>
        <h2>{t("config.sections_title")}</h2>
      </aside>
    </div>
  );
}
