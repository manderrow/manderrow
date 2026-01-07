import styles from "./Splashscreen.module.css";
import logo from "../../assets/Manderrow logo.svg";

import { t } from "../../i18n/i18n";

export default function Splashscreen() {
  return (
    <main class={styles.splashscreen}>
      <h1 class={styles.splashscreen__title}>Manderrow</h1>
      <p class={styles.splashscreen__text}>{t("splashscreen.launching_msg")}</p>

      <div class={styles.splashscreen__loader} aria-hidden></div>

      <img src={logo} alt="Manderrow logo" class={styles.splashscreen__logo} />
    </main>
  );
}
