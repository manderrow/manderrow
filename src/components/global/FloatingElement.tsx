import { Accessor, createSignal, JSX, onCleanup, onMount } from "solid-js";
import { useFloating, UseFloatingOptions } from "solid-floating-ui";
import { autoUpdate } from "@floating-ui/dom";

interface FloatingElementProps {
  content: string | JSX.Element;
  options?: UseFloatingOptions<HTMLElement, HTMLElement>;
  style?: JSX.CSSProperties;
  class?: JSX.HTMLAttributes<HTMLElement>["class"];
  classList?: JSX.CustomAttributes<HTMLElement>["classList"];
  children?: JSX.Element;
  hidden?: boolean;
  ref?: (element: HTMLElement) => void;
}
/**
 * Spawn a floating element. `props.content` is the content of the floating element, anchored to the first element passed into `props.children`.
 */
export function FloatingElement(props: FloatingElementProps) {
  const [reference, setReference] = createSignal<HTMLElement | null>();
  const [floating, setFloating] = createSignal<HTMLElement>();
  const [autoUpdateCleanup, setAutoUpdateCleanup] = createSignal<undefined | (() => void)>();

  // `position` is a reactive object
  const position = useFloating(reference, floating, props.options);

  onMount(() => {
    const floatingElement = floating();

    if (floatingElement) {
      if (props.ref) {
        props.ref(floatingElement);
      }

      setReference(floatingElement.previousSibling as HTMLElement); // previous sibling is always defined as seen in the structure below

      const cleanup = autoUpdate(reference()!, floatingElement, () => {
        position.update();
      });
      setAutoUpdateCleanup(() => cleanup);
    }
  });

  onCleanup(() => {
    autoUpdateCleanup()!(); // looks like a face
  });

  return (
    <>
      {props.children}

      <div
        class={props.class}
        classList={props.classList}
        ref={setFloating}
        style={{
          ...props.style,

          "pointer-events": props.hidden == null ? undefined : props.hidden ? "none" : "auto",
          visibility: props.hidden == null ? undefined : props.hidden ? "hidden" : "visible", // for ARIA only
          position: position.strategy,
          top: `${position.y ?? 0}px`,
          left: `${position.x ?? 0}px`,
        }}
      >
        {props.content}
      </div>
    </>
  );
}
