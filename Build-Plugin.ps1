$dist = New-Item -ItemType 'Directory' -Path (Join-Path $PSScriptRoot 'dist/io.github.mdonoughe.sbzdeck.sdPlugin') -Force

cargo build --release --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml' | Resolve-Path).ProviderPath
Join-Path $PSScriptRoot 'target\i686-pc-windows-msvc\release\sbzdeck.exe' | Copy-Item -Destination $dist.PSPath

# temporary
Join-Path $PSScriptRoot 'examples\sbzdeck.json' | Copy-Item -Destination $dist.PSPath

$manifest = Join-Path $PSScriptRoot 'manifest.json' | Get-Item | Get-Content | ConvertFrom-Json
$manifest.CodePathWin = 'sbzdeck.exe'
$manifest | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath ($dist | Join-Path -ChildPath 'manifest.json')

$images = @(
    $manifest.Actions | ForEach-Object { $_.Icon; $_.States | ForEach-Object { $_.Image } }
    $manifest.CategoryIcon
    $manifest.Icon
)
foreach ($image in $images) {
    foreach ($suffix in '.png', '@2x.png') {
        Join-Path $PSScriptRoot "${image}${suffix}" | Get-Item | Copy-Item -Destination $dist.PSPath
    }
}
