# Builds the app in release mode, then packs it into the setup wizard.
# Run from the repository root:   powershell -File installer\build-installer.ps1
# Result:  dist\ChargeDeck-Setup-<version>.exe  (+ a .sha256 checksum file)
#
# Needs: Rust (cargo) and Inno Setup 6   ->   winget install JRSoftware.InnoSetup
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

# The version lives in ONE place: Cargo.toml.
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
"Charge Deck $version"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
}
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# Find the Inno Setup compiler (per-user or system-wide install, or on PATH).
$candidates = @(
    (Get-Command iscc -ErrorAction SilentlyContinue).Source,
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) }
if (-not $candidates) { throw "Inno Setup 6 not found. Install it with: winget install JRSoftware.InnoSetup" }
$iscc = $candidates | Select-Object -First 1

& $iscc "/DAppVersion=$version" installer\charge-deck.iss
if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed" }

$setup = "dist\ChargeDeck-Setup-$version.exe"
$hash = (Get-FileHash $setup -Algorithm SHA256).Hash.ToLower()
"$hash  ChargeDeck-Setup-$version.exe" | Set-Content -Encoding ascii "$setup.sha256"
""
"Built $setup ($([math]::Round((Get-Item $setup).Length / 1MB, 1)) MB)"
"SHA-256: $hash"
