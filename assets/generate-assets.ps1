param()

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$assets = $PSScriptRoot
$purple = [System.Drawing.Color]::FromArgb(90, 52, 190)
$navy = [System.Drawing.Color]::FromArgb(23, 27, 45)
$white = [System.Drawing.Color]::White

function New-Canvas([int]$Width, [int]$Height, [string]$Path, [scriptblock]$Draw) {
    $bitmap = [System.Drawing.Bitmap]::new($Width, $Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        & $Draw $graphics $Width $Height
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Bmp)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

New-Canvas 493 58 (Join-Path $assets "banner.bmp") {
    param($g, $w, $h)
    $g.Clear($white)
    $purpleBrush = [System.Drawing.SolidBrush]::new($purple)
    $g.FillRectangle($purpleBrush, 0, 0, 122, $h)
    $font = [System.Drawing.Font]::new("Segoe UI Semibold", 18)
    $small = [System.Drawing.Font]::new("Segoe UI", 9)
    $g.DrawString("CUADRA", $font, [System.Drawing.Brushes]::White, 13, 7)
    $g.DrawString("POS AGENT", $small, [System.Drawing.Brushes]::White, 31, 35)
    $g.DrawString("Integracion segura de hardware POS", $small, [System.Drawing.Brushes]::DimGray, 145, 20)
    $purpleBrush.Dispose(); $font.Dispose(); $small.Dispose()
}

New-Canvas 493 312 (Join-Path $assets "dialog.bmp") {
    param($g, $w, $h)
    $g.Clear($navy)
    $purpleBrush = [System.Drawing.SolidBrush]::new($purple)
    $g.FillEllipse($purpleBrush, -90, 170, 330, 330)
    $font = [System.Drawing.Font]::new("Segoe UI Semibold", 28)
    $small = [System.Drawing.Font]::new("Segoe UI", 12)
    $g.DrawString("Cuadra", $font, [System.Drawing.Brushes]::White, 42, 48)
    $g.DrawString("POS Agent", $font, [System.Drawing.Brushes]::White, 42, 87)
    $g.DrawString("Tu hardware, conectado.", $small, [System.Drawing.Brushes]::Gainsboro, 47, 145)
    $purpleBrush.Dispose(); $font.Dispose(); $small.Dispose()
}

$iconBitmap = [System.Drawing.Bitmap]::new(256, 256)
$iconGraphics = [System.Drawing.Graphics]::FromImage($iconBitmap)
try {
    $iconGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $iconGraphics.Clear($navy)
    $brush = [System.Drawing.SolidBrush]::new($purple)
    $iconGraphics.FillEllipse($brush, 24, 24, 208, 208)
    $font = [System.Drawing.Font]::new("Segoe UI Black", 112, [System.Drawing.FontStyle]::Bold)
    $format = [System.Drawing.StringFormat]::new()
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    $iconGraphics.DrawString("C", $font, [System.Drawing.Brushes]::White, [System.Drawing.RectangleF]::new(0, 0, 256, 245), $format)
    $handle = $iconBitmap.GetHicon()
    $icon = [System.Drawing.Icon]::FromHandle($handle)
    $stream = [System.IO.File]::Create((Join-Path $assets "app.ico"))
    try { $icon.Save($stream) } finally { $stream.Dispose(); $icon.Dispose() }
    $brush.Dispose(); $font.Dispose(); $format.Dispose()
}
finally {
    $iconGraphics.Dispose()
    $iconBitmap.Dispose()
}

Write-Host "Recursos generados en $assets"
