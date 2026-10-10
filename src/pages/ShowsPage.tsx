import { useNavigate, useSearchParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { FolderPlus, Tv } from "lucide-react";
import { EmptyState, PageHeader, Segmented } from "@/components/EmptyState";
import { VirtualGrid } from "@/components/VirtualGrid";
import { Button } from "@/components/ui/button";
import { ShowCard } from "@/features/library/cards";
import { api } from "@/lib/api";
import { plural } from "@/lib/utils";

type Sort = "title" | "added" | "year";

export function ShowsPage() {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const sort = (params.get("sort") as Sort) ?? "title";
  const { data: count } = useQuery({ queryKey: ["shows", "count"], queryFn: () => api.shows({ limit: 1 }) });

  return (
    <div>
      <PageHeader
        title="TV Shows"
        subtitle={count ? plural(count.total, "show") : undefined}
        actions={
          <Segmented<Sort>
            value={sort}
            onChange={(v) => setParams(v === "title" ? {} : { sort: v }, { replace: true })}
            options={[
              { value: "title", label: "A–Z" },
              { value: "added", label: "Recently added" },
              { value: "year", label: "Year" },
            ]}
          />
        }
      />
      <div className="px-8">
        <VirtualGrid
          queryKey={["shows", sort]}
          fetchPage={(offset, limit) => api.shows({ offset, limit, sort })}
          itemAspect={1.5}
          extraHeight={52}
          minItemWidth={160}
          renderItem={(s) => <ShowCard show={s} />}
          empty={
            <EmptyState
              icon={<Tv />}
              title="No TV shows yet"
              description="Add a TV folder organised as Show / Season 01 / S01E01.mkv and Nsembe Play will build your shows automatically."
              action={
                <Button variant="primary" onClick={() => navigate("/settings")}>
                  <FolderPlus />
                  Add TV folder
                </Button>
              }
            />
          }
        />
      </div>
    </div>
  );
}
