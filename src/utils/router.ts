import { NavigateOptions, useLocation, useSearchParams } from "@solidjs/router";

export function replaceRouteState<T extends object>(mutator: (current: T) => T) {
  // solid router appears not to provide a way to do this, so here we go
  window.history.replaceState(mutator(window.history.state), "");
}

type SetSearchParamsFunction = ReturnType<typeof useSearchParams>[1];

// why couldn't they just export these type?
type ValidSearchParams<T> = SearchParams extends T ? T : never;
type SearchParams = { readonly [key in string]?: string | string[] };
type SetSearchParams = Parameters<SetSearchParamsFunction>[0];

export function useSearchParamsInPlace<T = SearchParams>(): [
  ValidSearchParams<T>,
  (params: T, options?: Partial<Omit<NavigateOptions, "replace" | "state">>) => void,
] {
  const [searchParams, setSearchParams] = useSearchParams();
  const location = useLocation();
  return [
    searchParams as ValidSearchParams<T>,
    (params, options) => {
      setSearchParams(params as SetSearchParams, { ...options, replace: true, state: location.state });
    },
  ];
}
