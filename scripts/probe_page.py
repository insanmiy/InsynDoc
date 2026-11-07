from PIL import Image

for name in ["orig-dark-editor-gemini1", "native-dark-editor-current"]:
    im = Image.open(r"D:\Github\InsynDoc\artifacts\ui\\" + name + ".png").convert("RGB")
    px = im.load()
    # page left edge ~x=346, scan y=300..320 horizontal gradient
    print(name)
    for x in range(340, 356):
        print("  x", x, px[x, 300])
    # top edge
    for y in range(170, 186):
        print("  y", y, px[600, y])
