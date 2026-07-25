// Host-provided `react` — served at the stable `/host/react.js` URL
// the tab.html import map names, so plugin bundles (built with react
// external) bind to the ONE React instance the host itself runs.
// Named exports are ENUMERATED (Object.keys of the installed
// package): vite's dev transform cannot statically expand
// `export *` from a CJS module and silently loses the names.
import React from "react";
export {
  Activity,
  Children,
  Component,
  Fragment,
  Profiler,
  PureComponent,
  StrictMode,
  Suspense,
  act,
  cache,
  cacheSignal,
  captureOwnerStack,
  cloneElement,
  createContext,
  createElement,
  createRef,
  forwardRef,
  isValidElement,
  lazy,
  memo,
  startTransition,
  // @ts-expect-error -- real runtime export the types omit.
  unstable_useCacheRefresh,
  use,
  useActionState,
  useCallback,
  useContext,
  useDebugValue,
  useDeferredValue,
  useEffect,
  useEffectEvent,
  useId,
  useImperativeHandle,
  useInsertionEffect,
  useLayoutEffect,
  useMemo,
  useOptimistic,
  useReducer,
  useRef,
  useState,
  useSyncExternalStore,
  useTransition,
  version,
} from "react";
export default React;
