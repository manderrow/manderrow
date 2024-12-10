import { JSX, Show, ErrorBoundary as SolidErrorBoundary } from "solid-js";
import Dialog from "./Dialog";

export default function ErrorBoundary(props: {
  children: JSX.Element;
}) {
  return <SolidErrorBoundary fallback={(err, reset) => <Error err={err} reset={reset} />}>
    {props.children}
  </SolidErrorBoundary>
}

function Error(props: { err: any, reset: () => void }) {
  return <Dialog>
    <h2>Oops!</h2>
    <p>An error occurred, but don't worry, we caught it for you.</p>
    <p>{props.err}</p>
    <Show when={props.err.stack}>
      {stack => <p>{stack()}</p>}
    </Show>

    <p>We're not perfect. That's why we invite you to <button>report</button> this error to us if you think we could do better.</p>

    <p>Otherwise, feel free to <button on:click={props.reset}>ignore</button> this error and carry on modding.</p>
  </Dialog>;
}