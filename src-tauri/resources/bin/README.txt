Optional bundled media tools
============================

Files placed in this folder are shipped with the installer and are found
automatically by Nsembe Play at runtime:

    mpv.exe       (plus its DLLs, e.g. from https://mpv.io/installation/)
    ffmpeg.exe
    ffprobe.exe

Run `npm run fetch-tools` (scripts/fetch-tools.ps1) to download them, or copy
them here manually. Without them the app still works: it falls back to the
built-in player and to filename/tag based metadata.
