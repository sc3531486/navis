Add-Type -AssemblyName System.Drawing

$sourcePath = "D:\myworkspace\Navis Go\extensions\navis-code\navis-code\ExtensionUI\assets\icon.png"
if (-not (Test-Path $sourcePath)) {
    $sourcePath = "D:\myworkspace\Navis Go\src-tauri\icons\icon_hd.png"
}

$destDir = "D:\myworkspace\Navis Go\src-tauri\icons"
$srcImg = [System.Drawing.Image]::FromFile($sourcePath)

function Resize-Image($image, $width, $height) {
    $destRect = New-Object System.Drawing.Rectangle(0, 0, $width, $height)
    $destImage = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($destImage)
    $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.DrawImage($image, $destRect, 0, 0, $image.Width, $image.Height, [System.Drawing.GraphicsUnit]::Pixel)
    $graphics.Dispose()
    return $destImage
}

# 1. 生成 32x32.png
$img32 = Resize-Image $srcImg 32 32
$img32.Save("$destDir\32x32.png", [System.Drawing.Imaging.ImageFormat]::Png)
$img32.Dispose()

# 2. 生成 64x64.png
$img64 = Resize-Image $srcImg 64 64
$img64.Save("$destDir\64x64.png", [System.Drawing.Imaging.ImageFormat]::Png)
$img64.Dispose()

# 3. 生成 128x128.png
$img128 = Resize-Image $srcImg 128 128
$img128.Save("$destDir\128x128.png", [System.Drawing.Imaging.ImageFormat]::Png)
$img128.Dispose()

# 4. 生成 128x128@2x.png (256x256)
$img256 = Resize-Image $srcImg 256 256
$img256.Save("$destDir\128x128@2x.png", [System.Drawing.Imaging.ImageFormat]::Png)

# 5. 生成标准 multi-frame ICO 包含 16, 32, 48, 64, 128, 256 像素尺寸
function Export-Ico($bitmaps, $outputPath) {
    $fs = [System.IO.File]::Create($outputPath)
    $bw = New-Object System.IO.BinaryWriter($fs)

    # ICONDIR Header
    $bw.Write([UInt16]0) # Reserved
    $bw.Write([UInt16]1) # Type (1 = ICO)
    $bw.Write([UInt16]$bitmaps.Count) # Image count

    $pngStreams = @()
    foreach ($bmp in $bitmaps) {
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $pngStreams += $ms
    }

    $offset = 6 + ($bitmaps.Count * 16)

    for ($i = 0; $i -lt $bitmaps.Count; $i++) {
        $b = $bitmaps[$i]
        $w = if ($b.Width -ge 256) { 0 } else { [byte]$b.Width }
        $h = if ($b.Height -ge 256) { 0 } else { [byte]$b.Height }
        $size = [UInt32]$pngStreams[$i].Length

        # ICONDIRENTRY
        $bw.Write([byte]$w)
        $bw.Write([byte]$h)
        $bw.Write([byte]0) # Colors
        $bw.Write([byte]0) # Reserved
        $bw.Write([UInt16]1) # Color planes
        $bw.Write([UInt16]32) # Bits per pixel
        $bw.Write([UInt32]$size) # Image size in bytes
        $bw.Write([UInt32]$offset) # Image offset

        $offset += $size
    }

    # Write image data
    for ($i = 0; $i -lt $bitmaps.Count; $i++) {
        $bytes = $pngStreams[$i].ToArray()
        $bw.Write($bytes)
        $pngStreams[$i].Dispose()
    }

    $bw.Flush()
    $bw.Close()
    $fs.Close()
}

$icoBitmaps = @(
    (Resize-Image $srcImg 16 16),
    (Resize-Image $srcImg 24 24),
    (Resize-Image $srcImg 32 32),
    (Resize-Image $srcImg 48 48),
    (Resize-Image $srcImg 64 64),
    (Resize-Image $srcImg 128 128),
    $img256
)

Export-Ico $icoBitmaps "$destDir\icon.ico"

foreach ($b in $icoBitmaps) {
    $b.Dispose()
}

$srcImg.Dispose()

Write-Output "Successfully generated ultra-HD 32x32, 64x64, 128x128, 128x128@2x, and multi-resolution icon.ico!"
