import { useEffect, type ReactNode } from "react";
import { createHashRouter, Navigate, RouterProvider, useRouteError } from "react-router";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { Toaster } from "sonner";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/misc";
import { AppLayout } from "@/layouts/AppLayout";
import { api } from "@/lib/api";
import { startEventBridge } from "@/lib/events";
import { AlbumDetailPage } from "@/pages/AlbumDetailPage";
import { AlbumsPage } from "@/pages/AlbumsPage";
import { ArtistDetailPage } from "@/pages/ArtistDetailPage";
import { FavoritesPage } from "@/pages/FavoritesPage";
import { GenreDetailPage } from "@/pages/GenreDetailPage";
import { HomePage } from "@/pages/HomePage";
import { MovieDetailPage } from "@/pages/MovieDetailPage";
import { MoviesPage } from "@/pages/MoviesPage";
import { MusicPage } from "@/pages/MusicPage";
import { PlaylistDetailPage } from "@/pages/PlaylistDetailPage";
import { PlaylistsPage } from "@/pages/PlaylistsPage";
import { RecentlyAddedPage, RecentlyPlayedPage } from "@/pages/RecentPages";
import { SearchPage } from "@/pages/SearchPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { ShowDetailPage } from "@/pages/ShowDetailPage";
import { ShowsPage } from "@/pages/ShowsPage";
import { WatchPage } from "@/pages/WatchPage";
import { WelcomePage } from "@/pages/WelcomePage";
import { useLibraryStore } from "@/stores/libraryStore";
import { usePlayerStore } from "@/stores/playerStore";
import { useUiStore } from "@/stores/uiStore";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, refetchOnWindowFocus: false, retry: 1 },
  },
});

function Splash() {
  return (
    <div className="grid h-full place-items-center bg-bg">
      <img src="/icon.svg" alt="" className="size-16 animate-pulse rounded-2xl" />
    </div>
  );
}

function Fatal({ message }: { message: string }) {
  return (
    <div className="grid h-full place-items-center bg-bg p-8">
      <div className="max-w-md text-center">
        <AlertTriangle className="mx-auto size-12 text-warning" />
        <h1 className="mt-4 font-display text-2xl font-bold">Something went wrong</h1>
        <p className="mt-2 text-[14px] text-muted">{message}</p>
        <Button variant="secondary" className="mt-6" onClick={() => window.location.reload()}>
          <RefreshCw />
          Reload
        </Button>
      </div>
    </div>
  );
}

function RouteError() {
  const error = useRouteError() as Error | undefined;
  return <Fatal message={error?.message ?? "This page failed to load."} />;
}

/** Sends first-time users to onboarding. */
function RequireOnboarding({ children }: { children: ReactNode }) {
  const { data: status } = useQuery({ queryKey: ["app-status"], queryFn: api.appStatus });
  if (!status) return <Splash />;
  if (!status.onboardingCompleted) return <Navigate to="/welcome" replace />;
  return <>{children}</>;
}

const router = createHashRouter([
  { path: "/welcome", element: <WelcomePage />, errorElement: <RouteError /> },
  {
    element: (
      <RequireOnboarding>
        <AppLayout />
      </RequireOnboarding>
    ),
    errorElement: <RouteError />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "movies", element: <MoviesPage /> },
      { path: "movies/:id", element: <MovieDetailPage /> },
      { path: "tv", element: <ShowsPage /> },
      { path: "tv/:id", element: <ShowDetailPage /> },
      { path: "music", element: <MusicPage /> },
      { path: "music/artists/:id", element: <ArtistDetailPage /> },
      { path: "music/genres/:name", element: <GenreDetailPage /> },
      { path: "albums", element: <AlbumsPage /> },
      { path: "albums/:id", element: <AlbumDetailPage /> },
      { path: "playlists", element: <PlaylistsPage /> },
      { path: "playlists/:id", element: <PlaylistDetailPage /> },
      { path: "favorites", element: <FavoritesPage /> },
      { path: "recent/added", element: <RecentlyAddedPage /> },
      { path: "recent/played", element: <RecentlyPlayedPage /> },
      { path: "search", element: <SearchPage /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "watch/:mediaId", element: <WatchPage /> },
      { path: "*", element: <Navigate to="/" replace /> },
    ],
  },
]);

/** Connects engine events and keeps engine selection in sync with Rust. */
function Bootstrap() {
  const { data: status, error } = useQuery({ queryKey: ["app-status"], queryFn: api.appStatus });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void startEventBridge(queryClient).then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    void api.scanStatus().then((s) => useLibraryStore.getState().setScan(s)).catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!status) return;
    useUiStore.getState().setEngines(status.engines);
    usePlayerStore.getState().setEngine(status.engines.audio);
  }, [status]);

  if (error) return <Fatal message={(error as Error).message} />;
  if (!status) return <Splash />;
  return <RouterProvider router={router} />;
}

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={450}>
        <Bootstrap />
        <Toaster
          theme="dark"
          position="bottom-right"
          offset={{ bottom: 100, right: 20 }}
          toastOptions={{
            classNames: {
              toast: "!bg-surface-2 !text-fg !border-line-strong !rounded-xl !shadow-pop",
              description: "!text-muted",
            },
          }}
        />
      </TooltipProvider>
    </QueryClientProvider>
  );
}
