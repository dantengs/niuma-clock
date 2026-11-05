# Puts cargo and the mingw clang driver on PATH for this session only, and (re)generates
# .cargo/config.toml from .cargo/config.example.toml.
# Keep this file ASCII: PowerShell 5.1 misreads UTF-8 without a BOM.
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }
$clangBin = $env:NIUMA_CLANG_BIN
if (-not $clangBin) { $clangBin = "C:\ghcup\ghc\9.10.3\mingw\bin" }
if (Test-Path $clangBin) { $env:PATH = "$clangBin;$env:PATH" }

$projectRoot = Split-Path -Parent $PSScriptRoot
$cargoDir = Join-Path $projectRoot ".cargo"
$config = Join-Path $cargoDir "config.toml"
$clang = Join-Path $clangBin "x86_64-w64-mingw32-clang.exe"
if (-not (Test-Path $clang)) { throw "clang not found: $clang (set NIUMA_CLANG_BIN to the llvm-mingw bin directory)" }
$sysroot = (& rustup run stable-x86_64-pc-windows-gnu rustc --print sysroot).Trim()
if ($LASTEXITCODE -ne 0) { throw "rustup toolchain stable-x86_64-pc-windows-gnu is not installed" }
$cargoHome = $env:CARGO_HOME
if (-not $cargoHome) { $cargoHome = Join-Path $env:USERPROFILE ".cargo" }
# Template lines are only substituted outside comments, so the generated file stays readable.
$lines = [IO.File]::ReadAllLines((Join-Path $cargoDir "config.example.toml")) | ForEach-Object {
    if ($_.TrimStart().StartsWith("#")) { $_ } else {
        $_.Replace("@CLANG@", $clang.Replace('\', '/')).Replace("@SYSROOT@", $sysroot.Replace('\', '/')).
           Replace("@CARGO_HOME@", $cargoHome.TrimEnd('\')).Replace("@ROOT@", $projectRoot.TrimEnd('\'))
    }
}
$text = ($lines -join "`n") + "`n"
if (-not (Test-Path $config) -or [IO.File]::ReadAllText($config) -ne $text) {
    [IO.File]::WriteAllText($config, $text, (New-Object Text.UTF8Encoding $false))
    Write-Host "Wrote $config"
}
