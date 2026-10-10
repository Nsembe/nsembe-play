import { useNavigate, useSearchParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Disc3, FolderPlus } from "lucide-react";
import { EmptyState, PageHeader, Segmented } from "@/components/EmptyState";
import { VirtualGrid } from "@/components/VirtualGrid";
import { Button } from "@/components/ui/button";
import { AlbumCard } from "@/features/library/cards";
import { api } from "@/lib/api";
import { plural } from "@/lib/utils";

type Sort = "title" | "artist" | "added" | "year";

export function AlbumsPage() {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const sort = (params.get("sort") as Sort) ?? "added";
  const { data: count } = useQuery({ queryKey: ["albums", "count"], queryFn: () => api.albums({ limit: 1 }) });
  return (
    <div>
      <PageHeader
        title="Albums"
        subtitle={count ? plural(count.total, "album") : undefined}
        actions={
          <Segmented<Sort>
            value={sort}
            onChange={(v) => setParams(v === "added" ? {} : { sort: v }, { replace: true })}
            options={[
              { value: "added", label: "Recently added" },
              { value: "title", label: "A–Z" },
              { value: "artist", label: "Artist" },
              { value: "year", label: "Year" },
            ]}
          />
        }
      />
      <div className="px-8">
        <VirtualGrid
          queryKey={["albums", sort]}
          fetchPage={(offset, limit) => api.albums({ offset, limit, sort })}
          itemAspect={1}
          extraHeight={76}
          minItemWidth={170}
          gap={8}
          renderItem={(a) => <AlbumCard album={a} />}
          empty={
            <EmptyState
              icon={<Disc3 />}
              title="No albums yet"
              description="Add a music folder to start building your library."
              action={
                <Button variant="primary" onClick={() => navigate("/settings")}>
                  <FolderPlus />
                  Add music folder
                </Button>
              }
            />
          }
        />
      </div>
    </div>
  );
}
