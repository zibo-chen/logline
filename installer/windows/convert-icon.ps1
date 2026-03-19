# Convert PNG to ICO for Windows installer
# Requires ImageMagick (available on GitHub Actions windows runners via choco)

param(
    [string]$InputPng = "res/icon.png",
    [string]$OutputIco = "res/icon.ico"
)

# Check if magick is available
if (Get-Command magick -ErrorAction SilentlyContinue) {
    magick $InputPng -define icon:auto-resize=256,128,64,48,32,16 $OutputIco
    Write-Host "Icon converted: $OutputIco"
} elseif (Get-Command convert -ErrorAction SilentlyContinue) {
    convert $InputPng -define icon:auto-resize=256,128,64,48,32,16 $OutputIco
    Write-Host "Icon converted: $OutputIco"
} else {
    Write-Error "ImageMagick not found. Install with: choco install imagemagick"
    exit 1
}
