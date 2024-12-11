import { A, useParams } from "@solidjs/router";

import styles from "./Error.module.css";

export default function Error() {
  const params = useParams();

  return (
    <main class={styles.error}>
      <h1 class={styles.error_heading}>404</h1>
      <p class={styles.error_path}>/{params.path}</p>
      <p class={styles.error_msg}>There is nothing on this page. You somehow got here...</p>
      <A href="/" tabindex="-1">
        <button>Go back home</button>
      </A>
    </main>
  );
}
