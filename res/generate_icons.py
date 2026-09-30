"""Generates the tray icons in res/ (design option A: tile with monogram).

Every size is drawn on its own (8x supersampled) instead of scaling one image, so the small tray sizes stay crisp.
Requires Pillow and the Segoe UI Bold font that ships with Windows. Run from the repository root:

    python res/generate_icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ACCENT = (6, 136, 156, 255)     # Neo teal
PAUSED = (122, 122, 122, 255)   # readable on light and dark taskbars
LOCK = (232, 145, 45, 255)      # Mod4 lock badge
WHITE = (255, 255, 255, 255)

SIZES = [16, 20, 24, 32, 40, 48, 64, 256]
SUPERSAMPLE = 8
FONT = Path("C:/Windows/Fonts/segoeuib.ttf")
OUT = Path(__file__).parent


def draw_icon(size, state):
    """state: active, paused, bypassed or lock. Coordinates are in a 16x16 grid like the design."""
    canvas = size * SUPERSAMPLE
    unit = canvas / 16
    image = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    box = [0.75 * unit, 0.75 * unit, 15.25 * unit, 15.25 * unit]
    radius = 3.5 * unit
    fill = {"active": ACCENT, "lock": ACCENT, "paused": PAUSED, "bypassed": None}[state]

    if fill:
        draw.rounded_rectangle(box, radius=radius, fill=fill)
    else:
        # Dashed outline: draw the outline, then cut gaps into the straight edges
        draw.rounded_rectangle(box, radius=radius, outline=ACCENT, width=round(1.5 * unit))
        gap, dash = 1.5 * unit, 2 * unit
        position = 4.5 * unit
        while position + gap < 11.5 * unit:
            position += dash
            for rect in (
                [position, 0, position + gap, 2.5 * unit],                  # top
                [position, 13.5 * unit, position + gap, canvas],            # bottom
                [0, position, 2.5 * unit, position + gap],                  # left
                [13.5 * unit, position, canvas, position + gap],            # right
            ):
                draw.rectangle(rect, fill=(0, 0, 0, 0))
            position += gap

    font = ImageFont.truetype(str(FONT), round(11 * unit))
    ink = ACCENT if state == "bypassed" else WHITE
    draw.text((8 * unit, 8.2 * unit), "N", font=font, fill=ink, anchor="mm")

    if state == "lock":
        # Transparent ring around the badge, so it reads on any taskbar colour
        ring = 4.3 * unit
        draw.ellipse([13 * unit - ring, 13 * unit - ring, 13 * unit + ring, 13 * unit + ring], fill=(0, 0, 0, 0))
        badge = 3.1 * unit
        draw.ellipse([13 * unit - badge, 13 * unit - badge, 13 * unit + badge, 13 * unit + badge], fill=LOCK)

    return image.resize((size, size), Image.LANCZOS)


def main():
    for state, name in [("active", "tray_active"), ("paused", "tray_paused"),
                        ("bypassed", "tray_bypassed"), ("lock", "tray_lock")]:
        images = [draw_icon(size, state) for size in SIZES]
        largest = images[-1]
        largest.save(OUT / f"{name}.ico", format="ICO", sizes=[(s, s) for s in SIZES], append_images=images[:-1])
        print(f"{name}.ico: {SIZES}")


if __name__ == "__main__":
    main()
