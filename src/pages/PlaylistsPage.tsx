import { useQuery } from "@tanstack/react-query";
import { ListMusic, Plus } from "lucide-react";
import { EmptyState, PageHeader } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/misc";
import { PlaylistCard } from "@/features/library/cards";
import { api } from "@/lib/api";
import { plural } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";

export function PlaylistsPage() {
  const openCreate = useUiStore((s) => s.openCreatePlaylist);
  const { data, isLoading } = useQuery({ queryKey: ["playlists"], queryFn: api.playlists });
  return (
    <div>
      <PageHeader
        title="Playlists"
        subtitle={data ? plural(data.length, "playlist") : undefined}
        actions={
          <Button variant="primary" onClick={() => openCreate()}>
            <Plus />
            New playlist
          </Button>
        }
      />
      <div className="px-6">
        {isLoading ? (
          <Skeleton className="mx-2 h-60" />
        ) : !data?.length ? (
          <EmptyState
            icon={<ListMusic />}
            title="No playlists yet"
            description="Create your first playlist."
            action={
              <Button variant="primary" onClick={() => openCreate()}>
                <Plus />
                Create playlist
              </Button>
            }
          />
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2">
            {data.map((p) => (
              <PlaylistCard key={p.id} playlist={p} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
