$dist = New-Item -ItemType 'Directory' -Path (Join-Path $PSScriptRoot 'dist/io.github.mdonoughe.sbzdeck.sdPlugin') -Force

Push-Location $PSScriptRoot
cargo build --release -p plugin
Get-Item target/i686-pc-windows-msvc/release/plugin.exe | Copy-Item -Destination (Join-Path $dist.PSPath 'sbzdeck.exe')
cargo web deploy --release -o dist/io.github.mdonoughe.sbzdeck.sdPlugin/inspector -p inspector
Pop-Location

$manifest = Join-Path $PSScriptRoot 'manifest.json' | Get-Item | Get-Content | ConvertFrom-Json
$manifest.CodePathWin = 'sbzdeck.exe'
$manifest.PropertyInspectorPath = 'inspector/index.html'
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
