#!/usr/bin/env pwsh
# Build RustyIP for a single target and place the binary in dist/ with the
# architecture (Rust target triple) in the filename.
#
# Usage: ./build.ps1 [-Target <rust-target>] [-SubName <name>] [-DistDir dist]
# If -Target is omitted, the host triple is used.
[CmdletBinding()]
param(
    [string]$Target = "",
    [string]$SubName = "",
    [string]$DistDir = "dist"
)
$ErrorActionPreference = "Stop"

if (-not $Target) {
    $hostLine = (& rustc -vV | Select-String '^host:').ToString()
    $Target = ($hostLine -replace '^host:\s*', '').Trim()
}

$ext = ""
if ($Target -like "*windows*") { $ext = ".exe" }
$suffix = ""
if ($SubName) { $suffix = "-$SubName" }
$out = "RustyIP-$Target$suffix$ext"

New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

Write-Host "[build] target=$Target subname=$SubName flags=$($env:RUSTFLAGS)"
& cargo build --release --target $Target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed for $Target" }

$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
$src = Join-Path $targetDir (Join-Path $Target (Join-Path "release" "RustyIP$ext"))
$dst = Join-Path $DistDir $out
Copy-Item $src $dst -Force
Write-Host "[build] -> $dst"
