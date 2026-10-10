import { NavLink } from "react-router";
import { useQuery } from "@tanstack/react-query";
import {
  Clapperboard,
  Disc3,
  Heart,
  History,
  Home,
  ListMusic,
  Music2,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Settings,
  Sparkles,
  Tv,
  type LucideIcon,
} from "lucide-react";
import { PlaylistCover } from "@/components/Artwork";
import { Tip } from "@/components/ui/misc";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { useUiStore } from "@/stores/uiStore";

const browse: { to: string; label: string; icon: LucideIcon; end?: boolean }[] = [
  { to: "/", label: "Home", icon: Home, end: true },
  { to: "/movies", label: "Movies", icon: Clapperboard },
  { to: "/tv", label: "TV Shows", icon: Tv },
  { to: "/music", label: "Music", icon: Music2 },
  { to: "/albums", label: "Albums", icon: Disc3 },
];

const library: { to: string; label: string; icon: LucideIcon }[] = [
  { to: "/playlists", label: "Playlists", icon: ListMusic },
  { to: "/favorites", label: "Favorites", icon: Heart },
  { to: "/recent/added", label: "Recently Added", icon: Sparkles },
  { to: "/recent/played", label: "Recently Played", icon: History },
];

function NavItem({ to, label, icon: Icon, end, collapsed }: { to: string; label: string; icon: LucideIcon; end?: boolean; collapsed: boolean }) {
  const link = (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) =>
        cn(
          "group relative flex h-10 items-center gap-3.5 rounded-lg px-3 text-[14px] font-medium transition-colors",
          collapsed && "justify-center px-0",
          isActive ? "bg-white/[0.07] text-fg" : "text-muted hover:bg-white/[0.04] hover:text-fg",
        )
      }
    >
      {({ isActive }) => (
        <>
          {isActive && <span className="absolute top-2 bottom-2 left-0 w-[3px] rounded-r-full bg-accent" />}
          <Icon className={cn("size-[19px] shrink-0", isActive && "text-accent")} strokeWidth={isActive ? 2.2 : 1.9} />
          {!collapsed && <span className="truncate">{label}</span>}
        </>
      )}
    </NavLink>
  );
  return collapsed ? (
    <Tip label={label} side="right">
      {link}
    </Tip>
  ) : (
    link
  );
}

export function Sidebar({ collapsed }: { collapsed: boolean }) {
  const toggle = useUiStore((s) => s.toggleSidebar);
  const openCreate = useUiStore((s) => s.openCreatePlaylist);
  const { data: playlists } = useQuery({ queryKey: ["playlists"], queryFn: api.playlists });

  return (
    <aside
      className={cn(
        "flex h-full shrink-0 flex-col border-r border-line bg-bg-deep/60 transition-[width] duration-200",
        collapsed ? "w-[76px]" : "w-[248px]",
      )}
    >
      <div className={cn("flex h-16 items-center gap-2.5 px-5", collapsed && "justify-center px-0")}>
        <img src="/icon.svg" alt="" className="size-8 rounded-lg" />
        {!collapsed && (
          <span className="font-display text-[17px] font-extrabold tracking-tight">
            Nsembe <span className="text-accent">Play</span>
          </span>
        )}
      </div>

      <nav className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto px-3 pb-3 scrollbar-none">
        {browse.map((n) => (
          <NavItem key={n.to} {...n} collapsed={collapsed} />
        ))}
        <div className={cn("mt-5 mb-1.5 px-3 text-[11px] font-semibold tracking-[0.12em] text-subtle uppercase", collapsed && "invisible h-px")}>
          Your Library
        </div>
        {library.map((n) => (
          <NavItem key={n.to} {...n} collapsed={collapsed} />
        ))}

        {!collapsed && (
          <>
            <div className="mt-5 mb-1 flex items-center justify-between px-3">
              <span className="text-[11px] font-semibold tracking-[0.12em] text-subtle uppercase">Playlists</span>
              <Tip label="New playlist">
                <button type="button" onClick={() => openCreate()} className="rounded-md p-1 text-muted hover:bg-white/[0.06] hover:text-fg">
                  <Plus className="size-4" />
                </button>
              </Tip>
            </div>
            {playlists?.slice(0, 12).map((p) => (
              <NavLink
                key={p.id}
                to={`/playlists/${p.id}`}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-3 rounded-lg px-2 py-1.5 text-[13px] transition-colors",
                    isActive ? "bg-white/[0.07] text-fg" : "text-muted hover:bg-white/[0.04] hover:text-fg",
                  )
                }
              >
                <PlaylistCover artworks={p.coverArtworks} title={p.name} className="size-8 shrink-0 rounded" />
                <span className="truncate">{p.name}</span>
              </NavLink>
            ))}
          </>
        )}
      </nav>

      <div className={cn("flex items-center gap-1 border-t border-line p-3", collapsed && "flex-col")}>
        <div className="min-w-0 flex-1">
          <NavItem to="/settings" label="Settings" icon={Settings} collapsed={collapsed} />
        </div>
        <Tip label={collapsed ? "Expand sidebar" : "Collapse sidebar"} side="right">
          <button type="button" onClick={toggle} className="grid size-10 place-items-center rounded-lg text-muted hover:bg-white/[0.05] hover:text-fg">
            {collapsed ? <PanelLeftOpen className="size-[18px]" /> : <PanelLeftClose className="size-[18px]" />}
          </button>
        </Tip>
      </div>
    </aside>
  );
}
