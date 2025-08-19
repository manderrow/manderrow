import styles from "./Splashscreen.module.css";
import logo from "../../assets/Manderrow logo.svg";

export default function Splashscreen() {
  return (
    <main class={styles.splashscreen}>
      <h1 class={styles.splashscreen__title}>Manderrow</h1>
      <p class={styles.splashscreen__text}>Launching...</p>
      <div class={styles.splashscreen__loader} aria-hidden></div>
      <img src={logo} alt="Manderrow logo" class={styles.splashscreen__logo} />
    </main>
  );
}
