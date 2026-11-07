from PIL import Image

for name in ["orig-dark-menu-search-gemini1", "native-dark-menu-search-current"]:
    im = Image.open(r"D:\Github\InsynDoc\artifacts\ui\\" + name + ".png").convert("RGB")
    px = im.load()
    rows = []
    for y in range(100, 800):
        cnt = 0
        for x in range(360, 920, 3):
            r, g, b = px[x, y]
            if 40 < r < 80 and 45 < g < 85 and 60 < b < 100 and b > r:
                cnt += 1
        if cnt > 120:
            rows.append(y)
    groups = []
    for y in rows:
        if groups and y - groups[-1][-1] <= 2:
            groups[-1].append(y)
        else:
            groups.append([y])
    print(name, "borders:", [(g[0], g[-1]) for g in groups])
    # last list row: scan x=400..900 for text px below y=620
    for y in range(615, 720):
        cnt = sum(1 for x in range(390, 900, 3) if px[x, y][0] > 140 and px[x, y][1] > 145)
        if cnt > 4:
            print("   text row", y, cnt)
