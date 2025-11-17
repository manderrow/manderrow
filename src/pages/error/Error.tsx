import { A, useParams } from "@solidjs/router";

import styles from "./Error.module.css";

import { t } from "../../i18n/i18n";

export default function Error() {
  const params = useParams();

  return (
    <main class={styles.error}>
      <h1 class={styles.error_heading}>404</h1>
      <p class={styles.error_path}>/{params.path}</p>
      <p class={styles.error_msg}>{t("404_page.subtitle")}</p>
      <A href="/profile/" tabindex="-1">
        <button>{t("404_page.home_btn")}</button>
      </A>
    </main>
  );
}
