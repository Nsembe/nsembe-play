import { useNavigate, useSearchParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Clapperboard, FolderPlus } from "lucide-react";
import { EmptyState, PageHeader, Segmented } from "@/components/EmptyState";
import { VirtualGrid } from "@/components/VirtualGrid";
import { Button } from "@/components/ui/button";
import { MovieCard } from "@/features/library/cards";
import { api } from "@/lib/api";
import { plural } from "@/lib/utils";

type Sort = "title" | "added" | "year" | "duration";

export function MoviesPage() {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const sort = (params.get("sort") as Sort) ?? "title";
  const genre = params.get("genre") ?? "";
  const { data: genres } = useQuery({ queryKey: ["movie-genres"], queryFn: api.movieGenres });
  const { data: count } = useQuery({ queryKey: ["movies", "count", genre], queryFn: () => api.movies({ limit: 1, genre }) });

  const set = (key: string, value: string) => {
    const next = new URLSearchParams(params);
    if (value) next.set(key, value);
    else next.delete(key);
    setParams(next, { replace: true });
  };

  return (
    <div>
      <PageHeader
        title="Movies"
        subtitle={count ? plural(count.total, "movie") : undefined}
        actions={
          <Segmented<Sort>
            value={sort}
            onChange={(v) => set("sort", v === "title" ? "" : v)}
            options={[
              { value: "title", label: "A–Z" },
              { value: "added", label: "Recently added" },
              { value: "year", label: "Release year" },
              { value: "duration", label: "Duration" },
            ]}
          />
        }
      >
        {genres && genres.length > 0 && (
          <div className="mt-5 flex gap-2 overflow-x-auto pb-1 scrollbar-none">
            {["", ...genres].map((g) => (
              <button
                key={g || "all"}
                type="button"
                onClick={() => set("genre", g)}
                className={
                  "h-8 shrink-0 rounded-full px-3.5 text-[13px] font-medium transition-colors " +
                  (genre === g ? "bg-accent text-forest" : "bg-white/[0.06] text-muted hover:text-fg")
                }
              >
                {g || "All genres"}
              </button>
            ))}
          </div>
        )}
      </PageHeader>
      <div className="px-8">
        <VirtualGrid
          queryKey={["movies", sort, genre]}
          fetchPage={(offset, limit) => api.movies({ offset, limit, sort, genre })}
          itemAspect={1.5}
          extraHeight={52}
          minItemWidth={160}
          renderItem={(m) => <MovieCard movie={m} />}
          empty={
            genre ? (
              <EmptyState icon={<Clapperboard />} title={`No ${genre} movies`} description="Try another genre." />
            ) : (
              <EmptyState
                icon={<Clapperboard />}
                title="No movies yet"
                description="Add your movie folder and Nsembe Play will organise it automatically."
                action={
                  <Button variant="primary" onClick={() => navigate("/settings")}>
                    <FolderPlus />
                    Add movie folder
                  </Button>
                }
              />
            )
          }
        />
      </div>
    </div>
  );
}
