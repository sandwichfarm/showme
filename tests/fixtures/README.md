These tiny synthetic fixtures are generated for showme's tests and contain no
third-party media:

- `red.ppm`: a 2×2 solid red image.
- `colors.qoi`: red and translucent blue pixels, 2×1 RGBA.
- `colors.svg` / `colors.svgz`: red and blue rectangles, 4×2; gzip uses mtime 0.
- `two-pages.pdf`: red then blue pages, each with a 3×2 media box.
- `short.mkv`: three red 4×2 FFV1 frames at 5 fps. Generated with:
  `ffmpeg -f lavfi -i color=c=red:s=4x2:r=5:d=0.6 -c:v ffv1 -pix_fmt bgr0 short.mkv`.

PNG, JPEG, and animated GIF fixtures are generated inside temporary test
directories so tests can vary pixels, alpha, cropping, and delays explicitly.
PDF tests require the pinned PDFium 7881 runtime and are run in CI with
`--include-ignored`; they must also be run explicitly when validating PDF support.
