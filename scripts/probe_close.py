from PIL import Image

im = Image.open(r"D:\Github\InsynDoc\artifacts\ui\native-dark-settings-current.png").convert("RGB")
px = im.load()
bg = (27, 30, 36)
for y in range(48, 100, 2):
    line = ""
    for x in range(858, 922, 2):
        p = px[x, y]
        d = abs(p[0] - bg[0]) + abs(p[1] - bg[1]) + abs(p[2] - bg[2])
        line += "#" if d > 120 else ("+" if d > 40 else ".")
    print(f"{y:3d} {line}")
