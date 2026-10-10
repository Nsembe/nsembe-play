<#
.SYNOPSIS
  Downloads mpv, ffmpeg and ffprobe for Windows into src-tauri/resources/bin so
  that `tauri build` bundles them with the installer and `tauri dev` finds them.

.NOTES
  - FFmpeg: "essentials" release build from https://www.gyan.dev/ffmpeg/builds/
  - mpv:    latest x86_64 build from https://github.com/shinchiro/mpv-winbuild-cmake
  Both are third-party binaries under (L)GPL licenses; review the licenses before
  redistributing them with your installer.
  Re-run with -Force to replace existing binaries.
#>
param([switch]$Force)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"   # much faster Invoke-WebRequest

$root = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $root "src-tauri\resources\bin"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "nsembe-play-tools"
New-Item -ItemType Directory -Force -Path $bin, $tmp | Out-Null

function Expand-Any($archive, $dest) {
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    if ($archive.EndsWith(".zip")) {
        Expand-Archive -Path $archive -DestinationPath $dest -Force
        return
    }
    # .7z: Windows 11's built-in tar (libarchive) reads 7z; fall back to 7-Zip.
    & tar.exe -xf $archive -C $dest 2>$null
    if ($LASTEXITCODE -ne 0) {
        $sevenZip = @("$env:ProgramFiles\7-Zip\7z.exe", "${env:ProgramFiles(x86)}\7-Zip\7z.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
        if (-not $sevenZip) { throw "Cannot extract $archive. Install 7-Zip or extract it manually into $bin." }
        & $sevenZip x $archive "-o$dest" -y | Out-Null
    }
}

# --- FFmpeg / FFprobe ---------------------------------------------------------
if ($Force -or -not (Test-Path (Join-Path $bin "ffprobe.exe"))) {
    Write-Host "Downloading FFmpeg (essentials build)..."
    $zip = Join-Path $tmp "ffmpeg.zip"
    Invoke-WebRequest -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" -OutFile $zip
    $out = Join-Path $tmp "ffmpeg"
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    Expand-Any $zip $out
    Get-ChildItem -Path $out -Recurse -Include ffmpeg.exe, ffprobe.exe | Copy-Item -Destination $bin -Force
    Write-Host "  ffmpeg.exe and ffprobe.exe installed."
} else {
    Write-Host "FFmpeg already present (use -Force to update)."
}

# --- mpv ---------------------------------------------------------------------
if ($Force -or -not (Test-Path (Join-Path $bin "mpv.exe"))) {
    Write-Host "Looking up the latest mpv build..."
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest" -Headers @{ "User-Agent" = "nsembe-play-setup" }
    # Plain x86_64 build (not -v3, not dev/ffmpeg packages) for maximum CPU compatibility.
    $asset = $release.assets | Where-Object { $_.name -match '^mpv-x86_64-\d{8}-git-.*\.7z$' } | Select-Object -First 1
    if (-not $asset) { throw "No suitable mpv asset found in $($release.html_url)" }
    Write-Host "Downloading $($asset.name)..."
    $archive = Join-Path $tmp $asset.name
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archive
    $out = Join-Path $tmp "mpv"
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    Expand-Any $archive $out
    $exe = Get-ChildItem -Path $out -Recurse -Filter mpv.exe | Select-Object -First 1
    if (-not $exe) { throw "mpv.exe not found in the archive" }
    # mpv.exe plus any DLLs that ship next to it.
    Get-ChildItem -Path $exe.DirectoryName -File | Where-Object { $_.Extension -in ".exe", ".dll" } | Copy-Item -Destination $bin -Force
    Write-Host "  mpv installed."
} else {
    Write-Host "mpv already present (use -Force to update)."
}

Write-Host ""
Write-Host "Done. Tools in ${bin}:"
Get-ChildItem $bin -Filter *.exe | ForEach-Object { Write-Host "  $($_.Name)" }
