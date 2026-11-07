from PIL import Image
import sys

def px(im, x, y):
    return im.getpixel((x, y))

a = Image.open(r'D:\Github\InsynDoc\artifacts\ui\orig-dark-editor-gemini1.png').convert('RGB')
b = Image.open(r'D:\Github\InsynDoc\artifacts\ui\native-dark-editor-pass2.png').convert('RGB')
for name, im in [('A', a), ('B', b)]:
    print(name, 'outline', px(im, 1203, 100))
    print(name, 'search', px(im, 37, 100))
    print(name, 'undo', px(im, 73, 100))
    print(name, 'align icon', px(im, 755, 100), 'caret?', px(im, 765, 100), px(im, 766, 101))
    print(name, 'stylechev', px(im, 217, 102), px(im, 218, 103), px(im, 219, 104))
    print(name, 'status', px(im, 60, 883), px(im, 500, 883))
    print(name, 'logo', px(im, 35, 35), px(im, 40, 40))
    print(name, 'sep', px(im, 159, 100), px(im, 160, 100))
