import sys
from PIL import Image

# Find bounding box of pixels matching a target color (approx) in a region.
path, r, g, b, tol = sys.argv[1], *map(int, sys.argv[2:5]), int(sys.argv[5])
x0, y0, x1, y1 = map(int, sys.argv[6:10])
im = Image.open(path).convert("RGB")
px = im.load()
hits = [
    (x, y)
    for y in range(y0, y1)
    for x in range(x0, x1)
    if abs(px[x, y][0] - r) <= tol
    and abs(px[x, y][1] - g) <= tol
    and abs(px[x, y][2] - b) <= tol
]
if hits:
    xs = [h[0] for h in hits]
    ys = [h[1] for h in hits]
    print("count", len(hits), "bbox x", min(xs), max(xs), "y", min(ys), max(ys))
else:
    print("no hits")
