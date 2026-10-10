import { createContext, useContext, type RefObject } from "react";

/** The main scrolling element of the layout, used by virtualized lists. */
export const ScrollContext = createContext<RefObject<HTMLElement | null>>({ current: null });

export function useScrollElement() {
  return useContext(ScrollContext);
}
