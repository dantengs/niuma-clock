# Builds the release exe and stamps the icon into it, then builds the installer.
# Output: dist\NiuMa.exe (portable) and dist\NiuMa-Setup.exe (installer)
# If dist\NiuMa.exe is running (file locked), writes dist\NiuMa.new.exe instead.
# Keep this file ASCII: PowerShell 5.1 misreads UTF-8 without a BOM.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. "$PSScriptRoot\env.ps1"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
$built = Join-Path $root "target\x86_64-pc-windows-gnullvm\release\niuma-clock.exe"
$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
$out = Join-Path $dist "NiuMa.exe"
try {
    Copy-Item $built $out -Force
} catch {
    $out = Join-Path $dist "NiuMa.new.exe"
    Write-Warning "dist\NiuMa.exe is in use; wrote $out instead. Quit NiuMa from the tray and rename it."
    Copy-Item $built $out -Force
}
# The built exe writes the icon into the copy (an exe cannot update its own resources).
function Stamp-Icon($target) {
    $p = Start-Process -FilePath $built -ArgumentList "--stamp-icon", "`"$target`"" -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "icon stamping failed for ${target}: $($p.ExitCode)" }
}
Stamp-Icon $out

# Installer: embeds the stamped exe above. Output: dist\NiuMa-Setup.exe
$env:NIUMA_PAYLOAD = $out
Push-Location (Join-Path $root "installer")
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "installer build failed" }
} finally {
    Pop-Location
    Remove-Item Env:NIUMA_PAYLOAD
}
$setup = Join-Path $dist "NiuMa-Setup.exe"
Copy-Item (Join-Path $root "installer\target\x86_64-pc-windows-gnullvm\release\niuma-setup.exe") $setup -Force
Stamp-Icon $setup

# Privacy check: the binaries must not contain local paths (user name, profile, checkout path).
# --remap-path-prefix in .cargo/config.toml rewrites them; this catches a misconfigured build.
$needles = @($env:USERNAME, $env:USERPROFILE, $root) | Where-Object { $_ } | ForEach-Object { $_.ToLower() }
foreach ($bin in $out, $setup) {
    $bytes = [IO.File]::ReadAllBytes($bin)
    $text = ([Text.Encoding]::ASCII.GetString($bytes) + [Text.Encoding]::Unicode.GetString($bytes)).ToLower()
    foreach ($n in $needles) {
        if ($text.Contains($n)) { throw "privacy check failed: $bin contains a local path or user name" }
    }
}
Write-Host "privacy check: no local paths in the binaries"
Get-Item $out, $setup | Select-Object FullName, Length
