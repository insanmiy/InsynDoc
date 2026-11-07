from PIL import Image

for name in ["orig-dark-settings-gemini1", "native-dark-settings-current"]:
    im = Image.open(r"D:\Github\InsynDoc\artifacts\ui\\" + name + ".png").convert("RGB")
    px = im.load()
    rows = []
    for y in range(60, 870):
        cnt = 0
        for x in range(380, 905, 2):
            r, g, b = px[x, y]
            if r > 140 and g > 145 and b > 155:
                cnt += 1
        if cnt > 8:
            rows.append(y)
    groups = []
    for y in rows:
        if groups and y - groups[-1][-1] <= 3:
            groups[-1].append(y)
        else:
            groups.append([y])
    print(name, [(g[0], g[-1]) for g in groups])
