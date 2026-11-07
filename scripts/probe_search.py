from PIL import Image

for name in ["orig-dark-menu-search-gemini1", "native-dark-menu-search-current"]:
    im = Image.open(r"D:\Github\InsynDoc\artifacts\ui\\" + name + ".png").convert("RGB")
    px = im.load()
    rows = []
    for y in range(300, 720):
        cnt = sum(1 for x in range(390, 900, 3) if px[x, y][0] > 140 and px[x, y][1] > 145)
        if cnt > 6:
            rows.append(y)
    groups = []
    for y in rows:
        if groups and y - groups[-1][-1] <= 4:
            groups[-1].append(y)
        else:
            groups.append([y])
    print(name, [(g[0], g[-1]) for g in groups])

print("--- settings toggles ---")
for name in ["orig-dark-settings-gemini1", "native-dark-settings-current"]:
    im = Image.open(r"D:\Github\InsynDoc\artifacts\ui\\" + name + ".png").convert("RGB")
    px = im.load()
    rows = []
    for y in range(60, 870):
        cnt = sum(
            1
            for x in range(840, 915)
            if px[x, y][2] > 200 and 120 < px[x, y][0] < 170 and px[x, y][1] > 160
        )
        if cnt > 5:
            rows.append(y)
    groups = []
    for y in rows:
        if groups and y - groups[-1][-1] <= 2:
            groups[-1].append(y)
        else:
            groups.append([y])
    print(name, [(g[0], g[-1]) for g in groups])
