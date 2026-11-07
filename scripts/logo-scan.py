import sys
from PIL import Image

for name in sys.argv[1:]:
    im = Image.open("artifacts/ui/" + name + ".png").convert("RGB")
    px = im.load()
    hits = [
        (x, y)
        for y in range(10, 70)
        for x in range(10, 70)
        if px[x, y][2] > 140 and px[x, y][2] > px[x, y][0] + 30
    ]
    if hits:
        xs = [h[0] for h in hits]
        ys = [h[1] for h in hits]
        cx, cy = (min(xs) + max(xs)) // 2, (min(ys) + max(ys)) // 2
        print(name, "bbox", min(xs), max(xs), min(ys), max(ys), "center", px[cx, cy])
    else:
        print(name, "no blue pixels")
