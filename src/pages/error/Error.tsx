import { A, useParams } from "@solidjs/router";
import { Show } from "solid-js";

export default function Error() {
  const params = useParams();
  const is404 = () => params.path != null;

  return (
    <main>
      <h1>Error</h1>
      <Show when={is404()} fallback={<p>A fatal error has occurred...</p>}>
        <p>This page does not exist.</p>
        <p>{params.path}</p>
        <A href="/">Go back home</A>
      </Show>
    </main>
  );
}
