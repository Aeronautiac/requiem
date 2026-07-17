# Builds lawliet-runtime as a Tauri sidecar and bundles Armonia for Windows.
# Run on a Windows machine that has the Rust MSVC toolchain, Node, and the
# WebView2 runtime installed. Produces an .msi / NSIS .exe under
# target\release\bundle\.
$ErrorActionPreference = "Stop"

# repo root = script dir
Set-Location -Path $PSScriptRoot

$triple = (rustc -vV | Select-String '^host: ') -replace 'host: ', ''
$triple = $triple.Trim()
$ext = if ($triple -like '*windows*') { '.exe' } else { '' }

Write-Host ">> Building lawliet-runtime ($triple)"
cargo build --release -p lawliet-runtime

$dest = "armonia\src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item "target\release\lawliet-runtime$ext" "$dest\lawliet-runtime-$triple$ext" -Force
Write-Host ">> Sidecar staged at $dest\lawliet-runtime-$triple$ext"

Write-Host ">> Bundling Armonia"
Set-Location armonia
npm install
npm run tauri build -- --config src-tauri/tauri.bundle.conf.json

Write-Host ">> Done. Bundles are in target\release\bundle\"
