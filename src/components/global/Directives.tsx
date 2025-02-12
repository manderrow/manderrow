import { onMount } from "solid-js";

type FocusableElement =
  | HTMLInputElement
  | HTMLSelectElement
  | HTMLTextAreaElement
  | HTMLAnchorElement
  | HTMLButtonElement
  | HTMLAreaElement;
export function autofocus(el: FocusableElement) {
  onMount(() => {
    el.focus();
  });
}
