# Copies only the public files (allowlist) into public-repo\, keeping its .git folder,
# then scans the result for private data. Run from anywhere.
# Keep this file ASCII: PowerShell 5.1 misreads UTF-8 without a BOM.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$pub = Join-Path $root "public-repo"
New-Item -ItemType Directory -Force $pub | Out-Null
Get-ChildItem $pub -Force | Where-Object { $_.Name -ne ".git" -and $_.Name -ne "target" -and $_.Name -ne ".cargo" } | Remove-Item -Recurse -Force
$files = @(
    ".gitignore", ".gitattributes", "Cargo.toml", "Cargo.lock", "LICENSE", "README.md",
    "CONTRIBUTORS.md", "SECURITY.md", "THIRD_PARTY_NOTICES.md",
    ".cargo\config.example.toml",
    "installer\Cargo.toml", "installer\Cargo.lock", "installer\build.rs"
)
# The development ledger has a non-ASCII name; match it by pattern to keep this file ASCII.
$files += (Get-ChildItem $root -File -Filter "*.md" | Where-Object { $_.Name -notmatch '^[\x00-\x7F]+$' } | ForEach-Object { $_.Name })
$dirs = @("src", "scripts", "installer\src", "installer\res", "third-party-licenses")
New-Item -ItemType Directory -Force (Join-Path $pub ".cargo") | Out-Null
Get-ChildItem (Join-Path $pub ".cargo") -Force | Where-Object { $_.Name -ne "config.toml" } | Remove-Item -Force
foreach ($f in $files) {
    $dst = Join-Path $pub $f
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    Copy-Item (Join-Path $root $f) $dst
}
foreach ($d in $dirs) {
    $dst = Join-Path $pub $d
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    Copy-Item (Join-Path $root $d) $dst -Recurse
}

# Android project: sources only. Build output, Gradle caches, the machine's SDK path
# (local.properties) and signing config / keys never leave this machine.
$androidSrc = Join-Path $root "android"
$androidSkip = '\\(build|\.gradle|\.idea|\.kotlin)(\\|$)|\\local\.properties$|\\keystore\.properties$|\.(jks|keystore)$'
Get-ChildItem $androidSrc -Recurse -File -Force | Where-Object { $_.FullName.Substring($androidSrc.Length) -notmatch $androidSkip } | ForEach-Object {
    $dst = Join-Path $pub ("android" + $_.FullName.Substring($androidSrc.Length))
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    Copy-Item $_.FullName $dst
}

# Privacy scan: user profile paths, drive paths outside Windows system folders, e-mail addresses
# other than GitHub noreply / anthropic noreply, and the local user name.
$user = $env:USERNAME
$patterns = @(
    # Allowed: C:\Windows\Fonts (system fonts) and C:\ghcup (the toolchain's documented default path).
    "(?i)" + [regex]::Escape($env:USERPROFILE), "(?i)\b" + [regex]::Escape($user) + "\b",
    "(?<![A-Za-z])[A-Z]:[\\/](?!Windows[\\/]Fonts|ghcup[\\/;])[^\s`"']*",
    "[A-Za-z0-9._%+-]+@(?!users\.noreply\.github\.com|anthropic\.com)[A-Za-z0-9.-]+\.[A-Za-z]{2,}"
)
$hits = @()
$scan = Get-ChildItem $pub -Recurse -File -Force | Where-Object { $_.FullName -notmatch '\\(\.git|target|third-party-licenses)\\' -and $_.FullName -notmatch '\\\.cargo\\config\.toml$' }
foreach ($file in $scan) {
    foreach ($p in $patterns) {
        $m = Select-String -Path $file.FullName -Pattern $p -Encoding utf8 -CaseSensitive
        foreach ($x in $m) { $hits += "$($file.FullName.Substring($pub.Length + 1)):$($x.LineNumber): $($x.Line.Trim())" }
    }
}
"files=$($scan.Count)"
if ($hits) { "PRIVACY HITS:"; $hits | Sort-Object -Unique } else { "privacy scan: clean" }
