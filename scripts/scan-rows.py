import sys
from PIL import Image

path = sys.argv[1]
x0, x1 = int(sys.argv[2]), int(sys.argv[3])
y0, y1 = int(sys.argv[4]), int(sys.argv[5])
im = Image.open(path).convert("RGB")
px = im.load()
rows = []
for y in range(y0, y1):
    n = sum(
        1
        for x in range(x0, x1, 8)
        if max(abs(px[x, y][c] - px[x, y + 1][c]) for c in range(3)) > 12
    )
    if n > (x1 - x0) // 8 * 0.5:
        rows.append(y)
# collapse consecutive
groups = []
for y in rows:
    if groups and y - groups[-1][-1] <= 1:
        groups[-1].append(y)
    else:
        groups.append([y])
print([g[len(g) // 2] for g in groups])
