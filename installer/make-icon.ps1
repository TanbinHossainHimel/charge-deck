# Draws the Charge Deck logo (red chamfered badge + dark bolt, from the design)
# and writes:
#   assets\charge-deck.ico   multi-size icon for the .exe, shortcuts and installer
#   assets\icon-256.png      256 px PNG used for the window / taskbar icon
# Run from the repository root:  powershell -File installer\make-icon.ps1
Add-Type -AssemblyName System.Drawing

$red  = [System.Drawing.ColorTranslator]::FromHtml("#FF2D46")
$dark = [System.Drawing.ColorTranslator]::FromHtml("#08090B")

function New-LogoPng([int]$s) {
    $bmp = New-Object System.Drawing.Bitmap $s, $s
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.Clear([System.Drawing.Color]::Transparent)

    # Chamfered badge: top-left and bottom-right corners are cut.
    $c = [single]($s * 0.26)
    $badge = @(
        (New-Object System.Drawing.PointF $c, 0),
        (New-Object System.Drawing.PointF $s, 0),
        (New-Object System.Drawing.PointF $s, ($s - $c)),
        (New-Object System.Drawing.PointF ($s - $c), $s),
        (New-Object System.Drawing.PointF 0, $s),
        (New-Object System.Drawing.PointF 0, $c)
    )
    $g.FillPolygon((New-Object System.Drawing.SolidBrush $red), [System.Drawing.PointF[]]$badge)

    # Lightning bolt on a 24x24 grid, scaled to ~62 % of the badge and centred.
    $grid = @(@(13,2), @(4,14), @(11,14), @(10,22), @(19,10), @(12,10))
    $k = $s * 0.62 / 24.0
    $off = ($s - 24.0 * $k) / 2.0
    $bolt = $grid | ForEach-Object { New-Object System.Drawing.PointF ([single]($off + $_[0] * $k)), ([single]($off + $_[1] * $k)) }
    $g.FillPolygon((New-Object System.Drawing.SolidBrush $dark), [System.Drawing.PointF[]]$bolt)
    $g.Dispose()

    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    return ,$ms.ToArray()
}

$sizes = 16, 24, 32, 48, 64, 128, 256
$pngs = @{}
foreach ($s in $sizes) { $pngs[$s] = New-LogoPng $s }

New-Item -ItemType Directory -Force assets | Out-Null

# Raw 64x64 RGBA pixels (4 bytes per pixel) for the running window's icon
# (taskbar, Alt-Tab). eframe takes raw pixels, so no image decoder is needed in the app.
$ms64 = New-Object System.IO.MemoryStream (, $pngs[64])
$bmp64 = New-Object System.Drawing.Bitmap $ms64
$raw = New-Object byte[] (64 * 64 * 4)
for ($y = 0; $y -lt 64; $y++) { for ($x = 0; $x -lt 64; $x++) {
    $c = $bmp64.GetPixel($x, $y); $i = ($y * 64 + $x) * 4
    $raw[$i] = $c.R; $raw[$i + 1] = $c.G; $raw[$i + 2] = $c.B; $raw[$i + 3] = $c.A
} }
[System.IO.File]::WriteAllBytes((Join-Path (Get-Location) "assets\icon-64.rgba"), $raw)
$bmp64.Dispose()
[System.IO.File]::WriteAllBytes((Join-Path (Get-Location) "assets\icon-256.png"), $pngs[256])

# ICO container with PNG images inside (supported since Windows Vista).
$out = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter $out
$w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
foreach ($s in $sizes) {
    $dim = if ($s -ge 256) { 0 } else { $s }
    $w.Write([byte]$dim); $w.Write([byte]$dim); $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([uint16]1); $w.Write([uint16]32)
    $w.Write([uint32]$pngs[$s].Length); $w.Write([uint32]$offset)
    $offset += $pngs[$s].Length
}
foreach ($s in $sizes) { $w.Write($pngs[$s]) }
$w.Flush()
[System.IO.File]::WriteAllBytes((Join-Path (Get-Location) "assets\charge-deck.ico"), $out.ToArray())
"Wrote assets\charge-deck.ico ($($out.Length) bytes) and assets\icon-256.png"
