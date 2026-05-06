from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "images" / "ui" / "menu"

CYAN = (87, 231, 255, 255)
CYAN_DIM = (42, 137, 168, 190)
CYAN_DARK = (10, 55, 70, 210)
WHITE = (205, 244, 248, 255)
GOLD = (255, 184, 78, 255)
PURPLE = (145, 104, 255, 235)
BG = (3, 15, 22, 0)


def glow(size: tuple[int, int], draw_fn, radius: int = 5) -> Image.Image:
    layer = Image.new("RGBA", size, BG)
    draw_fn(ImageDraw.Draw(layer), True)
    return layer.filter(ImageFilter.GaussianBlur(radius))


def make_icon(name: str, draw_fn, size: int = 128) -> None:
    img = Image.new("RGBA", (size, size), BG)
    img.alpha_composite(glow((size, size), draw_fn, 5))
    draw_fn(ImageDraw.Draw(img), False)
    img.save(OUT / f"{name}.png")


def line(draw: ImageDraw.ImageDraw, points, fill=CYAN, width=6) -> None:
    draw.line(points, fill=fill, width=width, joint="curve")


def rect(draw: ImageDraw.ImageDraw, box, outline=CYAN, fill=None, width=4) -> None:
    draw.rounded_rectangle(box, radius=5, outline=outline, fill=fill, width=width)


def nav_profile(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (36, 26, 92, 90), outline=c, fill=CYAN_DARK if not g else None, width=5)
    draw.ellipse((47, 38, 81, 70), outline=WHITE if not g else c, width=5)
    line(draw, [(52, 92), (46, 112), (82, 112), (76, 92)], c, 5)
    line(draw, [(28, 80), (18, 102)], c, 5)
    line(draw, [(100, 80), (110, 102)], c, 5)


def nav_inventory(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (37, 30, 91, 104), outline=c, fill=CYAN_DARK if not g else None, width=5)
    rect(draw, (45, 18, 83, 42), outline=c, width=5)
    line(draw, [(48, 58), (80, 58)], WHITE if not g else c, 5)
    line(draw, [(48, 76), (80, 76)], c, 4)
    line(draw, [(48, 94), (80, 94)], c, 4)


def nav_codex(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (28, 24, 94, 106), outline=c, fill=CYAN_DARK if not g else None, width=5)
    rect(draw, (42, 36, 102, 94), outline=c, width=4)
    line(draw, [(58, 50), (86, 50)], WHITE if not g else c, 4)
    line(draw, [(58, 64), (78, 64)], c, 4)
    draw.polygon([(64, 76), (74, 68), (84, 76), (74, 86)], outline=c, fill=None)


def nav_map(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.ellipse((24, 40, 104, 92), outline=c, width=5)
    line(draw, [(28, 86), (98, 42)], c, 5)
    line(draw, [(36, 58), (18, 34)], c, 4)
    line(draw, [(90, 74), (112, 100)], c, 4)
    draw.ellipse((56, 54, 72, 70), fill=WHITE if not g else c)


def nav_quests(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (34, 24, 94, 106), outline=c, fill=CYAN_DARK if not g else None, width=5)
    rect(draw, (48, 14, 80, 34), outline=c, width=4)
    for y in (48, 68, 88):
        line(draw, [(50, y), (80, y)], WHITE if y == 48 and not g else c, 5)
    line(draw, [(82, 96), (104, 74)], c, 5)


def nav_settings(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.ellipse((30, 30, 98, 98), outline=c, width=9)
    draw.ellipse((52, 52, 76, 76), fill=WHITE if not g else c)
    for x1, y1, x2, y2 in [(64, 8, 64, 28), (64, 100, 64, 120), (8, 64, 28, 64), (100, 64, 120, 64), (23, 23, 37, 37), (91, 91, 105, 105), (23, 105, 37, 91), (91, 37, 105, 23)]:
        line(draw, [(x1, y1), (x2, y2)], c, 7)


def action_equip(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    line(draw, [(32, 94), (88, 38)], c, 9)
    draw.polygon([(80, 30), (106, 22), (98, 48)], fill=WHITE if not g else c)
    line(draw, [(30, 42), (88, 100)], c, 7)


def action_skills(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    line(draw, [(36, 28), (98, 90)], c, 8)
    line(draw, [(92, 28), (30, 90)], c, 8)
    draw.ellipse((48, 48, 80, 80), outline=WHITE if not g else c, width=5)


def action_logs(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (34, 22, 94, 108), outline=c, fill=CYAN_DARK if not g else None, width=5)
    for y in (44, 62, 80):
        line(draw, [(48, y), (82, y)], WHITE if y == 44 and not g else c, 4)


def action_craft(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    line(draw, [(28, 96), (86, 38)], c, 10)
    draw.rectangle((78, 22, 106, 50), outline=c, width=5)
    line(draw, [(38, 34), (98, 94)], c, 7)


def action_comms(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.rounded_rectangle((24, 34, 96, 82), radius=18, outline=c, fill=CYAN_DARK if not g else None, width=5)
    draw.polygon([(44, 80), (36, 106), (66, 82)], fill=c)
    for x in (44, 62, 80):
        draw.ellipse((x - 4, 56, x + 4, 64), fill=WHITE if not g else c)


def action_save(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (32, 24, 96, 106), outline=c, fill=CYAN_DARK if not g else None, width=5)
    draw.rectangle((46, 26, 82, 50), fill=WHITE if not g else c)
    rect(draw, (46, 70, 82, 100), outline=c, width=4)


def action_return(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = (200, 132, 48, 180) if g else GOLD
    rect(draw, (42, 24, 92, 104), outline=c, width=6)
    line(draw, [(80, 64), (24, 64)], c, 9)
    draw.polygon([(24, 64), (50, 42), (50, 86)], fill=c)


def attr_survival(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.ellipse((34, 30, 70, 66), outline=c, width=5)
    draw.ellipse((58, 30, 94, 66), outline=c, width=5)
    line(draw, [(36, 58), (64, 100), (92, 58)], WHITE if not g else c, 6)


def attr_mobility(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (42, 20, 74, 82), outline=c, width=5)
    rect(draw, (36, 78, 94, 106), outline=c, fill=CYAN_DARK if not g else None, width=5)
    line(draw, [(74, 36), (96, 54)], c, 5)


def attr_scanning(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.ellipse((28, 28, 100, 100), outline=c, width=5)
    line(draw, [(64, 18), (64, 110)], c, 4)
    line(draw, [(18, 64), (110, 64)], c, 4)
    draw.ellipse((56, 56, 72, 72), fill=WHITE if not g else c)


def attr_gather(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    line(draw, [(30, 96), (90, 36)], c, 8)
    line(draw, [(56, 36), (96, 36), (96, 58)], WHITE if not g else c, 6)
    line(draw, [(42, 82), (62, 102)], c, 7)


def attr_analysis(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.ellipse((34, 34, 94, 92), outline=c, width=5)
    line(draw, [(46, 70), (34, 104)], c, 5)
    line(draw, [(82, 70), (96, 104)], c, 5)
    line(draw, [(48, 58), (82, 58)], WHITE if not g else c, 5)


def stat_armor(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    draw.polygon([(64, 18), (100, 34), (92, 86), (64, 110), (36, 86), (28, 34)], outline=c, fill=CYAN_DARK if not g else None)
    line(draw, [(64, 28), (64, 98)], WHITE if not g else c, 5)


def stat_carry(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = CYAN_DIM if g else CYAN
    rect(draw, (38, 36, 90, 102), outline=c, fill=CYAN_DARK if not g else None, width=5)
    rect(draw, (48, 24, 80, 44), outline=c, width=5)
    line(draw, [(50, 64), (78, 64)], WHITE if not g else c, 5)


def resource_crystal(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = (38, 120, 170, 180) if g else (85, 215, 255, 255)
    draw.polygon([(64, 12), (92, 48), (74, 114), (38, 104), (30, 44)], fill=c)
    draw.polygon([(64, 12), (64, 108), (92, 48)], fill=(160, 245, 255, 190) if not g else c)
    line(draw, [(38, 104), (64, 108), (74, 114)], WHITE if not g else c, 3)


def resource_coin(draw: ImageDraw.ImageDraw, g: bool) -> None:
    c = (190, 120, 44, 180) if g else GOLD
    draw.ellipse((24, 24, 104, 104), outline=c, fill=(80, 45, 12, 210) if not g else None, width=8)
    draw.ellipse((42, 42, 86, 86), outline=(255, 226, 128, 255) if not g else c, width=5)
    line(draw, [(64, 48), (64, 82)], c, 5)


def thumbnail_base() -> Image.Image:
    img = Image.new("RGBA", (192, 256), (3, 13, 22, 255))
    d = ImageDraw.Draw(img)
    for x in range(0, 192, 16):
        d.line((x, 0, x, 256), fill=(14, 49, 65, 110))
    for y in range(0, 256, 16):
        d.line((0, y, 192, y), fill=(14, 49, 65, 90))
    return img


def save_thumb(name: str, draw_fn) -> None:
    img = thumbnail_base()
    d = ImageDraw.Draw(img)
    draw_fn(d)
    glow_layer = img.filter(ImageFilter.GaussianBlur(2))
    img = Image.alpha_composite(glow_layer, img)
    img.save(OUT / f"{name}.png")


def thumb_alien(d: ImageDraw.ImageDraw) -> None:
    d.ellipse((52, 76, 138, 158), fill=(74, 54, 92, 255), outline=PURPLE, width=4)
    d.ellipse((70, 96, 88, 116), fill=(180, 248, 255, 255))
    d.ellipse((106, 96, 124, 116), fill=(180, 248, 255, 255))
    for x in (52, 76, 104, 130):
        d.line((x, 150, x - 24, 210), fill=(120, 88, 190, 230), width=8)
    d.polygon([(48, 96), (22, 70), (66, 80)], fill=(132, 92, 205, 230))
    d.polygon([(134, 96), (174, 70), (124, 80)], fill=(132, 92, 205, 230))
    d.line((48, 64, 134, 64), fill=CYAN, width=3)


def thumb_relic(d: ImageDraw.ImageDraw) -> None:
    for i, box in enumerate([(78, 44, 116, 196), (46, 82, 76, 204), (120, 84, 150, 204)]):
        d.rounded_rectangle(box, radius=4, fill=(22, 76, 92, 255), outline=CYAN, width=3)
        for y in range(box[1] + 14, box[3] - 10, 28):
            d.line((box[0] + 6, y, box[2] - 6, y), fill=(120, 245, 255, 190), width=2)
    d.polygon([(96, 18), (116, 44), (78, 44)], fill=(120, 245, 255, 230))
    d.rectangle((38, 204, 158, 218), fill=(34, 34, 44, 255), outline=GOLD)


def thumb_star(d: ImageDraw.ImageDraw) -> None:
    d.ellipse((50, 70, 148, 168), fill=(20, 23, 62, 255), outline=(118, 140, 255, 230), width=3)
    for y in range(82, 156, 16):
        d.arc((50, y - 38, 148, y + 38), 0, 360, fill=(84, 180, 255, 160), width=2)
    for x, y, r in [(34, 46, 2), (154, 54, 3), (42, 192, 2), (164, 178, 2), (96, 38, 3)]:
        d.ellipse((x - r, y - r, x + r, y + r), fill=(222, 236, 255, 255))
    d.line((34, 198, 164, 58), fill=(160, 92, 255, 170), width=3)


def thumb_archive(d: ImageDraw.ImageDraw) -> None:
    d.rounded_rectangle((52, 38, 142, 208), radius=8, fill=(18, 70, 86, 255), outline=CYAN, width=4)
    d.rounded_rectangle((66, 58, 128, 146), radius=4, fill=(8, 34, 48, 255), outline=(116, 241, 255, 180), width=3)
    for y in (74, 92, 110, 128, 166, 184):
        d.line((76, y, 118, y), fill=(136, 244, 255, 210), width=3)
    d.polygon([(140, 54), (164, 70), (148, 88)], fill=GOLD)
    d.line((50, 218, 142, 218), fill=(82, 120, 130, 200), width=4)


def make_contact_sheet(files: list[Path]) -> None:
    cell = 96
    cols = 8
    rows = (len(files) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * cell, rows * cell), (5, 15, 24, 255))
    for i, file in enumerate(files):
        img = Image.open(file).convert("RGBA")
        img.thumbnail((cell - 16, cell - 16), Image.Resampling.LANCZOS)
        x = (i % cols) * cell + (cell - img.width) // 2
        y = (i // cols) * cell + (cell - img.height) // 2
        sheet.alpha_composite(img, (x, y))
    sheet.save(OUT / "_menu_asset_contact_sheet.png")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    icons = {
        "nav_profile": nav_profile,
        "nav_inventory": nav_inventory,
        "nav_codex": nav_codex,
        "nav_map": nav_map,
        "nav_quests": nav_quests,
        "nav_settings": nav_settings,
        "action_equip": action_equip,
        "action_skills": action_skills,
        "action_logs": action_logs,
        "action_craft": action_craft,
        "action_comms": action_comms,
        "action_save": action_save,
        "action_return": action_return,
        "attr_survival": attr_survival,
        "attr_mobility": attr_mobility,
        "attr_scanning": attr_scanning,
        "attr_gathering": attr_gather,
        "attr_analysis": attr_analysis,
        "stat_health": attr_survival,
        "stat_stamina": attr_mobility,
        "stat_armor": stat_armor,
        "stat_carry": stat_carry,
        "resource_crystal": resource_crystal,
        "resource_coin": resource_coin,
        "brand_crystal": resource_crystal,
    }
    for name, fn in icons.items():
        make_icon(name, fn)

    save_thumb("codex_alien_life", thumb_alien)
    save_thumb("codex_relic_tech", thumb_relic)
    save_thumb("codex_star_geography", thumb_star)
    save_thumb("codex_civilization", thumb_archive)

    make_contact_sheet(sorted(OUT.glob("*.png")))
    print(f"generated {len(list(OUT.glob('*.png')))} menu assets in {OUT}")


if __name__ == "__main__":
    main()
