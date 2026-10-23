# Draws the branded artwork for the setup wizard:
#   assets\wizard-large.bmp   side panel on the Welcome and Finish pages
#   assets\wizard-small.bmp   small logo in the header of the other pages
# Run from the repository root:  powershell -File installer\make-wizard-images.ps1
Add-Type -AssemblyName System.Drawing

$red   = [System.Drawing.ColorTranslator]::FromHtml("#FF2D46")
$dark  = [System.Drawing.ColorTranslator]::FromHtml("#08090B")
$panel = [System.Drawing.ColorTranslator]::FromHtml("#101216")
$muted = [System.Drawing.ColorTranslator]::FromHtml("#9AA0AA")

# Rajdhani (the app's font, bundled in assets\fonts) for the wordmark.
$fonts = New-Object System.Drawing.Text.PrivateFontCollection
$fonts.AddFontFile((Resolve-Path "assets\fonts\Rajdhani-Bold.ttf").Path)
$family = $fonts.Families[0]

function Draw-Badge($g, [single]$x, [single]$y, [single]$size) {
    $c = [single]($size * 0.26)
    $pts = [System.Drawing.PointF[]]@(
        (New-Object System.Drawing.PointF ($x + $c), $y),
        (New-Object System.Drawing.PointF ($x + $size), $y),
        (New-Object System.Drawing.PointF ($x + $size), ($y + $size - $c)),
        (New-Object System.Drawing.PointF ($x + $size - $c), ($y + $size)),
        (New-Object System.Drawing.PointF $x, ($y + $size)),
        (New-Object System.Drawing.PointF $x, ($y + $c))
    )
    $g.FillPolygon((New-Object System.Drawing.SolidBrush $red), $pts)
    $k = $size * 0.62 / 24.0
    $off = ($size - 24.0 * $k) / 2.0
    $bolt = @(@(13,2), @(4,14), @(11,14), @(10,22), @(19,10), @(12,10)) | ForEach-Object {
        New-Object System.Drawing.PointF ([single]($x + $off + $_[0] * $k)), ([single]($y + $off + $_[1] * $k))
    }
    $g.FillPolygon((New-Object System.Drawing.SolidBrush $dark), [System.Drawing.PointF[]]$bolt)
}

# ---- Large side panel: 2x size (328 x 628), Inno scales it for the screen DPI ----
$w = 328; $h = 628
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'; $g.TextRenderingHint = 'AntiAlias'
$g.Clear($dark)
Draw-Badge $g 84 150 160
$sf = New-Object System.Drawing.StringFormat; $sf.Alignment = 'Center'
$big = New-Object System.Drawing.Font $family, 38, ([System.Drawing.FontStyle]::Bold), 'Pixel'
$g.DrawString("CHARGE", $big, [System.Drawing.Brushes]::White, (New-Object System.Drawing.RectangleF 0, 340, $w, 60), $sf)
$g.DrawString("DECK", $big, (New-Object System.Drawing.SolidBrush $red), (New-Object System.Drawing.RectangleF 0, 385, $w, 60), $sf)
$small = New-Object System.Drawing.Font $family, 15, ([System.Drawing.FontStyle]::Bold), 'Pixel'
$g.DrawString("ALL YOUR WIRELESS GEAR", $small, (New-Object System.Drawing.SolidBrush $muted), (New-Object System.Drawing.RectangleF 0, 470, $w, 30), $sf)
$g.DrawString("ONE BATTERY DECK", $small, (New-Object System.Drawing.SolidBrush $muted), (New-Object System.Drawing.RectangleF 0, 495, $w, 30), $sf)
# thin red line at the bottom
$g.FillRectangle((New-Object System.Drawing.SolidBrush $red), 0, ($h - 6), $w, 6)
$g.Dispose()
$bmp.Save((Join-Path (Get-Location) "assets\wizard-large.bmp"), [System.Drawing.Imaging.ImageFormat]::Bmp)
$bmp.Dispose()

# ---- Small header logo: 2x size (110 x 110) on a white background ----
$bmp = New-Object System.Drawing.Bitmap 110, 110
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.Color]::White)
Draw-Badge $g 15 15 80
$g.Dispose()
$bmp.Save((Join-Path (Get-Location) "assets\wizard-small.bmp"), [System.Drawing.Imaging.ImageFormat]::Bmp)
$bmp.Dispose()
"Wrote assets\wizard-large.bmp and assets\wizard-small.bmp"
