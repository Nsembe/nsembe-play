import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Input, Textarea } from "@/components/ui/misc";
import { api } from "@/lib/api";
import { useUiStore } from "@/stores/uiStore";
import type { Playlist } from "@/types";

function PlaylistForm({
  initial,
  submitLabel,
  onSubmit,
}: {
  initial?: { name: string; description: string };
  submitLabel: string;
  onSubmit: (name: string, description: string) => Promise<void>;
}) {
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [busy, setBusy] = useState(false);
  return (
    <form
      className="mt-5 flex flex-col gap-3"
      onSubmit={async (e) => {
        e.preventDefault();
        if (!name.trim()) return;
        setBusy(true);
        try {
          await onSubmit(name, description);
        } finally {
          setBusy(false);
        }
      }}
    >
      <label className="text-[12.5px] font-medium text-muted" htmlFor="pl-name">
        Name
      </label>
      <Input id="pl-name" autoFocus maxLength={120} value={name} onChange={(e) => setName(e.target.value)} placeholder="My playlist" />
      <label className="mt-1 text-[12.5px] font-medium text-muted" htmlFor="pl-desc">
        Description <span className="text-subtle">(optional)</span>
      </label>
      <Textarea id="pl-desc" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="Add a short description" />
      <div className="mt-3 flex justify-end">
        <Button type="submit" variant="primary" disabled={!name.trim() || busy}>
          {submitLabel}
        </Button>
      </div>
    </form>
  );
}

/** Global "New playlist" dialog; optionally adds pending items once created. */
export function CreatePlaylistDialog() {
  const { open, mediaIds } = useUiStore((s) => s.createPlaylist);
  const close = useUiStore((s) => s.closeCreatePlaylist);
  const qc = useQueryClient();
  const navigate = useNavigate();
  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent>
        <DialogTitle>New playlist</DialogTitle>
        <DialogDescription>
          {mediaIds.length > 0 ? `The selected item${mediaIds.length > 1 ? "s" : ""} will be added to it.` : "Collect your favourite songs, movies and episodes."}
        </DialogDescription>
        {open && (
          <PlaylistForm
            submitLabel="Create"
            onSubmit={async (name, description) => {
              try {
                const p = await api.createPlaylist(name, description);
                if (mediaIds.length) await api.addToPlaylist(p.id, mediaIds);
                void qc.invalidateQueries({ queryKey: ["playlists"] });
                void qc.invalidateQueries({ queryKey: ["home"] });
                close();
                toast.success(`Created ${p.name}`);
                if (!mediaIds.length) navigate(`/playlists/${p.id}`);
              } catch (e) {
                toast.error("Couldn't create playlist", { description: (e as Error).message });
              }
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

export function EditPlaylistDialog({ playlist, open, onOpenChange }: { playlist: Playlist; open: boolean; onOpenChange: (o: boolean) => void }) {
  const qc = useQueryClient();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogTitle>Edit details</DialogTitle>
        {open && (
          <PlaylistForm
            initial={{ name: playlist.name, description: playlist.description ?? "" }}
            submitLabel="Save"
            onSubmit={async (name, description) => {
              try {
                await api.updatePlaylist(playlist.id, name, description);
                void qc.invalidateQueries();
                onOpenChange(false);
              } catch (e) {
                toast.error("Couldn't save playlist", { description: (e as Error).message });
              }
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void | Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!open) setBusy(false);
  }, [open]);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>{description}</DialogDescription>
        <div className="mt-6 flex justify-end gap-2">
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              await onConfirm();
              onOpenChange(false);
            }}
          >
            {confirmLabel}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
