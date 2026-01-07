import {
  closestCenter,
  createSortable,
  DragDropProvider,
  DragDropSensors,
  DragEventHandler,
  DragOverlay,
  Id,
  SortableProvider,
  useDragDropContext,
} from "@thisbeyond/solid-dnd";
import { createSignal, For, JSX, Signal } from "solid-js";

import styles from "./SortableList.module.css";

function SortableListItem(props: { item: JSX.Element; id: Id }) {
  const sortable = createSortable(props.id);
  const [state] = useDragDropContext()!;
  return (
    <div
      // @ts-ignore: typescript is unaware of solid's use: syntax
      use:sortable
      classList={{
        [styles.opacity25]: sortable.isActiveDraggable,
        [styles.transitionTransform]: !!state.active.draggable,
      }}
    >
      {props.item}
    </div>
  );
}

export function SortableList<T>({
  items: [items, setItems],
  id,
  children,
}: {
  items: Signal<readonly T[]>;
  id: (item: T, index: number) => Id;
  children: (item: T, index: number) => JSX.Element;
}) {
  const [activeItem, setActiveItem] = createSignal<Id | null>(null);
  const ids = () => items().map(id);

  const onDragStart: DragEventHandler = ({ draggable }) => setActiveItem(draggable.id);

  const onDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (draggable && droppable) {
      const currentItems = items();
      const fromIndex = currentItems.findIndex((item, i) => id(item, i) === draggable.id);
      const toIndex = currentItems.findIndex((item, i) => id(item, i) === droppable.id);
      if (fromIndex !== toIndex) {
        const updatedItems = currentItems.slice();
        updatedItems.splice(toIndex, 0, ...updatedItems.splice(fromIndex, 1));
        setItems(updatedItems);
      }
    }
  };

  return (
    <DragDropProvider onDragStart={onDragStart} onDragEnd={onDragEnd} collisionDetector={closestCenter}>
      <DragDropSensors />
      <SortableProvider ids={ids()}>
        <For each={items()}>{(item, i) => <SortableListItem item={children(item, i())} id={id(item, i())} />}</For>
      </SortableProvider>
      <DragOverlay class={styles.dragOverlay}>{activeItem()}</DragOverlay>
    </DragDropProvider>
  );
}
