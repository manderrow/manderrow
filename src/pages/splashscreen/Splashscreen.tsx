import styles from "./Splashscreen.module.css";

export default function Splashscreen() {
  return (
    <div class={styles.splashscreen}>
      <h1 class={styles.splashscreen__title}>Manderrow</h1>
      <p class={styles.splashscreen__text}>Launching...</p>
      <div class={styles.splashscreen__loader} aria-hidden></div>
      {/* <img src="/img/logo512.png" alt="Logo" class="splashscreen__logo-image" /> */}
    </div>
  );
}
