package main

import (
	"fmt"
	"image"
	"image/color"
	"image/png"
	"math"
	"os"
)

func clamp(v float64, min, max float64) float64 {
	if v < min {
		return min
	}
	if v > max {
		return max
	}
	return v
}

// removeBackground cleans the background using color distance and smooth alpha interpolation.
// isDarkBg: true for primary (dark background), false for secondary (light background).
func removeBackground(src image.Image, isDarkBg bool) *image.NRGBA {
	bounds := src.Bounds()
	w, h := bounds.Dx(), bounds.Dy()
	dst := image.NewNRGBA(image.Rect(0, 0, w, h))

	var bgR, bgG, bgB float64
	if isDarkBg {
		// Sample corner
		r, g, b, _ := src.At(0, 0).RGBA()
		bgR, bgG, bgB = float64(r>>8), float64(g>>8), float64(b>>8)
	} else {
		r, g, b, _ := src.At(0, 0).RGBA()
		bgR, bgG, bgB = float64(r>>8), float64(g>>8), float64(b>>8)
	}

	lowThresh := 25.0
	highThresh := 65.0

	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			c := src.At(bounds.Min.X+x, bounds.Min.Y+y)
			r, g, b, _ := c.RGBA()
			rf, gf, bf := float64(r>>8), float64(g>>8), float64(b>>8)

			dr := rf - bgR
			dg := gf - bgG
			db := bf - bgB
			dist := math.Sqrt(dr*dr + dg*dg + db*db)

			if dist <= lowThresh {
				// Pure background
				dst.SetNRGBA(x, y, color.NRGBA{0, 0, 0, 0})
			} else if dist >= highThresh {
				// Pure foreground
				dst.SetNRGBA(x, y, color.NRGBA{uint8(rf), uint8(gf), uint8(bf), 255})
			} else {
				// Smooth edge transition
				alpha := (dist - lowThresh) / (highThresh - lowThresh)
				// De-matting to remove background color bleed
				fgR := clamp((rf-(1-alpha)*bgR)/alpha, 0, 255)
				fgG := clamp((gf-(1-alpha)*bgG)/alpha, 0, 255)
				fgB := clamp((bf-(1-alpha)*bgB)/alpha, 0, 255)

				dst.SetNRGBA(x, y, color.NRGBA{
					R: uint8(fgR),
					G: uint8(fgG),
					B: uint8(fgB),
					A: uint8(alpha * 255),
				})
			}
		}
	}

	return dst
}

// cropTight trims transparent borders and creates a square centered image with a slight margin
func cropTight(src *image.NRGBA, marginRatio float64) *image.NRGBA {
	b := src.Bounds()
	minX, minY, maxX, maxY := b.Dx(), b.Dy(), 0, 0

	for y := 0; y < b.Dy(); y++ {
		for x := 0; x < b.Dx(); x++ {
			c := src.NRGBAAt(x, y)
			if c.A > 15 {
				if x < minX {
					minX = x
				}
				if x > maxX {
					maxX = x
				}
				if y < minY {
					minY = y
				}
				if y > maxY {
					maxY = y
				}
			}
		}
	}

	if minX > maxX || minY > maxY {
		return src
	}

	fgW := maxX - minX + 1
	fgH := maxY - minY + 1
	maxDim := fgW
	if fgH > maxDim {
		maxDim = fgH
	}

	margin := int(float64(maxDim) * marginRatio)
	totalDim := maxDim + margin*2

	dst := image.NewNRGBA(image.Rect(0, 0, totalDim, totalDim))

	// Center the foreground within dst
	offsetX := (totalDim-fgW)/2 - minX
	offsetY := (totalDim-fgH)/2 - minY

	for y := minY; y <= maxY; y++ {
		for x := minX; x <= maxX; x++ {
			c := src.NRGBAAt(x, y)
			dst.SetNRGBA(x+offsetX, y+offsetY, c)
		}
	}

	return dst
}

// resizeBilinear simple bilinear downsampler
func resizeBilinear(src *image.NRGBA, targetW, targetH int) *image.NRGBA {
	dst := image.NewNRGBA(image.Rect(0, 0, targetW, targetH))
	srcW := src.Bounds().Dx()
	srcH := src.Bounds().Dy()

	xRatio := float64(srcW-1) / float64(targetW)
	yRatio := float64(srcH-1) / float64(targetH)

	for y := 0; y < targetH; y++ {
		for x := 0; x < targetW; x++ {
			gx := xRatio * float64(x)
			gy := yRatio * float64(y)
			gxi := int(gx)
			gyi := int(gy)
			xDiff := gx - float64(gxi)
			yDiff := gy - float64(gyi)

			c00 := src.NRGBAAt(gxi, gyi)
			c10 := src.NRGBAAt(gxi+1, gyi)
			c01 := src.NRGBAAt(gxi, gyi+1)
			c11 := src.NRGBAAt(gxi+1, gyi+1)

			blend := func(a, b, c, d uint8) uint8 {
				v := float64(a)*(1-xDiff)*(1-yDiff) +
					float64(b)*xDiff*(1-yDiff) +
					float64(c)*(1-xDiff)*yDiff +
					float64(d)*xDiff*yDiff
				return uint8(clamp(v, 0, 255))
			}

			dst.SetNRGBA(x, y, color.NRGBA{
				R: blend(c00.R, c10.R, c01.R, c11.R),
				G: blend(c00.G, c10.G, c01.G, c11.G),
				B: blend(c00.B, c10.B, c01.B, c11.B),
				A: blend(c00.A, c10.A, c01.A, c11.A),
			})
		}
	}
	return dst
}

func savePNG(path string, img image.Image) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return png.Encode(f, img)
}

func main() {
	os.MkdirAll("ui/assets", 0755)

	// 1. Process primary logo (white/light ship on dark background -> for dark themes)
	fPri, err := os.Open("assets/branding/logos/vessel_logo_primary.png")
	if err != nil {
		panic(err)
	}
	imgPri, _, _ := image.Decode(fPri)
	fPri.Close()

	transparentPri := removeBackground(imgPri, true)
	croppedPri := cropTight(transparentPri, 0.06) // 6% margin
	savePNG("ui/assets/logo-light.png", croppedPri)
	fmt.Println("Saved ui/assets/logo-light.png (dimensions:", croppedPri.Bounds().Dx(), "x", croppedPri.Bounds().Dy(), ")")

	// 2. Process secondary logo (dark ship on light background -> for light themes)
	fSec, err := os.Open("assets/branding/logos/vessel_logo_secondary.png")
	if err != nil {
		panic(err)
	}
	imgSec, _, _ := image.Decode(fSec)
	fSec.Close()

	transparentSec := removeBackground(imgSec, false)
	croppedSec := cropTight(transparentSec, 0.06)
	savePNG("ui/assets/logo-dark.png", croppedSec)
	fmt.Println("Saved ui/assets/logo-dark.png (dimensions:", croppedSec.Bounds().Dx(), "x", croppedSec.Bounds().Dy(), ")")

	// 3. Favicon (64x64 and 128x128)
	fav64 := resizeBilinear(croppedPri, 64, 64)
	savePNG("ui/assets/favicon.png", fav64)
	savePNG("ui/favicon.png", fav64)
	fmt.Println("Saved ui/assets/favicon.png and ui/favicon.png (64x64)")

	fav128 := resizeBilinear(croppedPri, 128, 128)
	savePNG("ui/assets/favicon-128.png", fav128)

	favDark64 := resizeBilinear(croppedSec, 64, 64)
	savePNG("ui/assets/favicon-dark.png", favDark64)
	fmt.Println("Done generating branding assets!")
}
