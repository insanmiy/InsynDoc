from fontTools import ttLib
from fontTools.varLib.instancer import instantiateVariableFont

for weight, name in [(400, "Inter-Regular"), (500, "Inter-Medium"), (600, "Inter-SemiBold")]:
    font = ttLib.TTFont("public/interface-fonts/Inter-Variable.ttf")
    instantiateVariableFont(font, {"wght": weight}, inplace=True)
    font.save(f"native/fonts/{name}.ttf")
    print(name, "saved")
