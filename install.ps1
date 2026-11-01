# Installs phpz: irm https://raw.githubusercontent.com/CaduVarela/phpz/master/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repository = "CaduVarela/phpz"
$installDirectory = if ($env:PHPZ_INSTALL_DIR) { $env:PHPZ_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "phpz\bin" }
$version = if ($env:PHPZ_VERSION) { $env:PHPZ_VERSION } else { "latest" }

if ($env:PHPZ_RELEASE_URL) {
    $baseUrl = $env:PHPZ_RELEASE_URL
} elseif ($version -eq "latest") {
    $baseUrl = "https://github.com/$repository/releases/latest/download"
} else {
    $baseUrl = "https://github.com/$repository/releases/download/$version"
}

if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    throw "phpz install: unsupported architecture $($env:PROCESSOR_ARCHITECTURE) (x64 only)"
}

$asset = "phpz-windows-x64.exe"
$workDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("phpz-install-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Path $workDirectory | Out-Null

try {
    Write-Host "Downloading $asset"
    $download = Join-Path $workDirectory $asset
    Invoke-WebRequest -Uri "$baseUrl/$asset" -OutFile $download -UseBasicParsing
    $sumsFile = Join-Path $workDirectory "SHA256SUMS"
    Invoke-WebRequest -Uri "$baseUrl/SHA256SUMS" -OutFile $sumsFile -UseBasicParsing

    $line = (Get-Content $sumsFile) | Where-Object { $_ -match " $([regex]::Escape($asset))\s*$" } | Select-Object -First 1
    if (-not $line) { throw "phpz install: no checksum is listed for $asset" }
    $expected = ($line -split "\s+")[0].ToLower()
    $actual = (Get-FileHash -Algorithm SHA256 $download).Hash.ToLower()
    if ($actual -ne $expected) { throw "phpz install: the checksum of the download does not match, nothing was installed" }
    Write-Host "Verified SHA-256 checksum"

    New-Item -ItemType Directory -Force -Path $installDirectory | Out-Null
    $target = Join-Path $installDirectory "phpz.exe"
    Copy-Item -Force $download $target
    Write-Host "Installed $target"
} finally {
    Remove-Item -Recurse -Force $workDirectory -ErrorAction SilentlyContinue
}

# Put the install directory on the user PATH, keeping the value's type and every other entry.
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
try {
    $current = $key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $kind = if ($key.GetValueNames() -contains "Path") { $key.GetValueKind("Path") } else { [Microsoft.Win32.RegistryValueKind]::ExpandString }
    $entries = $current -split ";" | Where-Object { $_ -ne "" }
    if ($entries -notcontains $installDirectory) {
        $key.SetValue("Path", (($entries + $installDirectory) -join ";"), $kind)
        Write-Host "Added $installDirectory to your user PATH"
    }
} finally {
    $key.Close()
}

& $target setup
Write-Host "Done. Open a new terminal and run: phpz install 8.3"
