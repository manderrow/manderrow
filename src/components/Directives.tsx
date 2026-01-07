import { createRenderEffect, onMount, Signal } from "solid-js";

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

export function bindValue(el: HTMLInputElement, value: () => Signal<string>) {
  const [field, setField] = value();
  createRenderEffect(() => (el.value = field()))
  el.addEventListener("input", (e) => setField((e.target! as HTMLInputElement).value))
};
