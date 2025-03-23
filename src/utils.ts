import { batch, createSignal } from "solid-js";

export const numberFormatter = new Intl.NumberFormat();
export const roundedNumberFormatter = new Intl.NumberFormat(undefined, {
  maximumSignificantDigits: 3,
  notation: "compact",
});
export function humanizeFileSize(sizeBytes: number, space = false): string {
  const i = sizeBytes === 0 ? 0 : Math.floor(Math.log(sizeBytes) / Math.log(1000));
  return (sizeBytes / Math.pow(1000, i)).toFixed(1) + (space ? " " : "") + ["B", "KB", "MB", "GB", "TB"][i];
}

export const removeProperty = <Obj, Prop extends keyof Obj>(obj: Obj, prop: Prop): Omit<Obj, Prop> => {
  const { [prop]: _, ...rest } = obj;

  return rest;
};

export function createSignalResource<T>(initialValue: () => Promise<T>) {
  const [getT, _setT] = createSignal<T>();
  const [getError, setError] = createSignal<unknown>();

  const setT = (t: T) => {
    // this is how solid checks if it's a function, so we'll do the same
    if (typeof t === "function") {
      _setT(() => t);
    } else {
      _setT(t as Exclude<T, Function>);
    }
  };

  const loading = () => getT() === undefined && getError() === undefined;

  const loaded = (async () => {
    let t: T;
    try {
      t = await initialValue();
    } catch (e) {
      if (loading()) {
        setError(e);
      } else {
        console.error(e);
      }
      throw e;
    }
    if (loading()) {
      setT(t);
    }
    return t;
  })();

  return {
    get state() {
      if (loading()) {
        return "pending";
      } else {
        return "ready";
      }
    },
    get loading() {
      return loading();
    },
    get latest() {
      const e = getError();
      if (e != null) throw e;
      return getT();
    },
    get error() {
      return getError();
    },
    get latestOrThrow() {
      const t = getT();
      if (t === undefined) {
        throw getError() ?? new Error("Resource is not loaded");
      }
      return t;
    },
    loaded,
    set value(value: T) {
      batch(() => {
        setT(value);
        setError(undefined);
      });
    },
  };
}
