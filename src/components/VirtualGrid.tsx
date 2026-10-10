import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useScrollElement } from "@/layouts/scrollContext";
import { Skeleton } from "@/components/ui/misc";
import type { Page } from "@/types";

interface VirtualGridProps<T> {
  queryKey: unknown[];
  fetchPage: (offset: number, limit: number) => Promise<Page<T>>;
  renderItem: (item: T, index: number) => ReactNode;
  /** Minimum card width; columns are derived from the available width. */
  minItemWidth?: number;
  /** Artwork height / width (1.5 for posters, 1 for squares). */
  itemAspect: number;
  /** Height of everything below the artwork (title, subtitle, padding). */
  extraHeight: number;
  gap?: number;
  pageSize?: number;
  empty: ReactNode;
}

/**
 * Grid that only renders visible rows and loads pages on demand, so libraries
 * with thousands of items never sit in React state or the DOM all at once.
 */
export function VirtualGrid<T>({
  queryKey,
  fetchPage,
  renderItem,
  minItemWidth = 168,
  itemAspect,
  extraHeight,
  gap = 20,
  pageSize = 90,
  empty,
}: VirtualGridProps<T>) {
  const scrollRef = useScrollElement();
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [scrollMargin, setScrollMargin] = useState(0);

  const query = useInfiniteQuery({
    queryKey,
    queryFn: ({ pageParam }) => fetchPage(pageParam, pageSize),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.offset + last.items.length < last.total ? last.offset + last.items.length : undefined),
  });

  useLayoutEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const measure = () => {
      setWidth(el.clientWidth);
      const scroller = scrollRef.current;
      if (scroller) {
        setScrollMargin(el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop);
      }
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [scrollRef, query.isLoading]);

  const items = query.data?.pages.flatMap((p) => p.items) ?? [];
  const total = query.data?.pages[0]?.total ?? 0;
  const columns = Math.max(1, Math.floor((width + gap) / (minItemWidth + gap)));
  const itemWidth = width > 0 ? (width - gap * (columns - 1)) / columns : minItemWidth;
  const rowHeight = Math.round(itemWidth * itemAspect + extraHeight + gap);
  const rowCount = Math.ceil(total / columns);

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight,
    overscan: 3,
    scrollMargin,
  });

  useEffect(() => {
    virtualizer.measure();
  }, [rowHeight, columns, virtualizer]);

  const virtualRows = virtualizer.getVirtualItems();
  const lastRow = virtualRows[virtualRows.length - 1];
  useEffect(() => {
    if (!lastRow) return;
    const needed = (lastRow.index + 1) * columns;
    if (needed >= items.length && query.hasNextPage && !query.isFetchingNextPage) {
      void query.fetchNextPage();
    }
  }, [lastRow, columns, items.length, query]);

  if (query.isLoading) {
    return (
      <div ref={containerRef} className="grid gap-5" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${minItemWidth}px, 1fr))` }}>
        {Array.from({ length: 12 }, (_, i) => (
          <div key={i}>
            <Skeleton style={{ aspectRatio: `1 / ${itemAspect}` }} />
            <Skeleton className="mt-3 h-3.5 w-3/4" />
          </div>
        ))}
      </div>
    );
  }
  if (total === 0) return <>{empty}</>;

  return (
    <div ref={containerRef} className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
      {virtualRows.map((row) => (
        <div
          key={row.key}
          className="absolute top-0 left-0 grid w-full"
          style={{
            transform: `translateY(${row.start - scrollMargin}px)`,
            gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
            columnGap: gap,
          }}
        >
          {Array.from({ length: columns }, (_, c) => {
            const idx = row.index * columns + c;
            if (idx >= total) return null;
            const item = items[idx];
            return item !== undefined ? (
              <div key={idx}>{renderItem(item, idx)}</div>
            ) : (
              <div key={idx}>
                <Skeleton style={{ aspectRatio: `1 / ${itemAspect}` }} />
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}
