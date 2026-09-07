"""
Generates the Stasis app icon.

The mark is four viewfinder corner brackets framing a single copper square.
Reasoning, so the decision is not lost:

  - Not an eye. An eye mark on a product whose pitch is "this is not spyware"
    argues against itself, and eye logos are the most worn-out shape in the
    category.
  - Brackets read as framing and holding rather than aiming. There is no
    cross, so it does not become a crosshair and does not narrow the product to
    a shooter overlay.
  - The centre square is the same glyph the status bar uses as its state mark,
    so the icon and the interface share one motif instead of two.
  - Colours and the lit top edge come straight from DESIGN.md.

Rendered at 4x and downsampled, because GDI-style hard edges alias badly at
icon sizes. Run: python tools/make-icon.py
"""

from PIL import Image, ImageDraw

SIZE = 1024
SS = 4  # supersample factor

BG = (11, 10, 9, 255)  # --surface-base
BRACKET = (232, 228, 222, 255)  # --text-primary
COPPER = (184, 115, 51, 255)  # --copper-fill

# Proportions are tuned for the 16px case, not for the 1024px one. Anything
# thinner than roughly a tenth of the canvas disappears in a taskbar, so the
# strokes are heavier and the gaps wider than they would be in a poster.
# The app's lit-edge motif is deliberately absent: at icon scale it stopped
# reading as a bevel and became a white stripe across the top.
# Weights compared side by side at 256, 32 and 16px before choosing. Heavier
# strokes went chunky at large sizes; lighter ones thinned out in the taskbar.
CORNER_RADIUS = 176
FRAME_INSET = 172  # from canvas edge to the outside of the brackets
ARM = 270  # length of each bracket arm
STROKE = 84
MARK = 216  # side of the centre square


def draw_icon(size: int) -> Image.Image:
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    k = s / SIZE  # scale factor from the 1024 design grid

    def px(v: float) -> float:
        return v * k

    # Tile
    d.rounded_rectangle(
        [0, 0, s - 1, s - 1], radius=px(CORNER_RADIUS), fill=BG
    )

    inset = px(FRAME_INSET)
    arm = px(ARM)
    st = px(STROKE)
    right = s - inset
    bottom = s - inset

    # Four corner brackets. Square ends, no rounding: machined, not soft.
    # top-left
    d.rectangle([inset, inset, inset + arm, inset + st], fill=BRACKET)
    d.rectangle([inset, inset, inset + st, inset + arm], fill=BRACKET)
    # top-right
    d.rectangle([right - arm, inset, right, inset + st], fill=BRACKET)
    d.rectangle([right - st, inset, right, inset + arm], fill=BRACKET)
    # bottom-left
    d.rectangle([inset, bottom - st, inset + arm, bottom], fill=BRACKET)
    d.rectangle([inset, bottom - arm, inset + st, bottom], fill=BRACKET)
    # bottom-right
    d.rectangle([right - arm, bottom - st, right, bottom], fill=BRACKET)
    d.rectangle([right - st, bottom - arm, right, bottom], fill=BRACKET)

    # The observed mark, in the one accent colour.
    half = px(MARK) / 2
    c = s / 2
    d.rectangle([c - half, c - half, c + half, c + half], fill=COPPER)

    return img.resize((size, size), Image.LANCZOS)


if __name__ == "__main__":
    import pathlib

    out = pathlib.Path(__file__).resolve().parent.parent / "app-icon.png"
    draw_icon(SIZE).save(out)
    print(f"wrote {out}")

    # A contact sheet at real icon sizes, to check the mark still reads small.
    sheet_sizes = [16, 24, 32, 48, 64, 128, 256]
    pad = 16
    width = sum(x + pad for x in sheet_sizes) + pad
    height = max(sheet_sizes) + pad * 2
    sheet = Image.new("RGBA", (width, height), (40, 38, 36, 255))
    x = pad
    for n in sheet_sizes:
        sheet.paste(draw_icon(n), (x, (height - n) // 2))
        x += n + pad
    sheet_path = pathlib.Path(__file__).resolve().parent.parent / "app-icon-sizes.png"
    sheet.save(sheet_path)
    print(f"wrote {sheet_path}")
