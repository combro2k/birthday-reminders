import os
import struct
import zlib

ROOT = os.path.join(os.path.dirname(__file__), "..", "static")
os.makedirs(ROOT, exist_ok=True)


def _chunk(tag: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(tag + data) & 0xFFFFFFFF
    return struct.pack("!I", len(data)) + tag + data + struct.pack("!I", crc)


def write_png(path: str, width: int, height: int, pixels: bytearray) -> None:
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        row_start = y * width * 4
        raw.extend(pixels[row_start : row_start + width * 4])

    ihdr = struct.pack("!IIBBBBB", width, height, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), level=9)
    png = b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", ihdr) + _chunk(b"IDAT", idat) + _chunk(b"IEND", b"")

    with open(path, "wb") as f:
        f.write(png)


def make_icon(width: int, height: int, maskable: bool = False) -> bytearray:
    pixels = bytearray(width * height * 4)
    for y in range(height):
        for x in range(width):
            i = (y * width + x) * 4
            t = (x + y) / max(1, (width + height - 2))
            r = int(15 + 26 * t)
            g = int(118 + 66 * t)
            b = int(110 + 55 * t)

            radius = int(min(width, height) * (0.22 if not maskable else 0.06))
            cx = x if x < width // 2 else width - 1 - x
            cy = y if y < height // 2 else height - 1 - y
            in_corner = cx < radius and cy < radius and (cx - radius) ** 2 + (cy - radius) ** 2 > radius ** 2
            if not maskable and in_corner:
                r, g, b = 245, 245, 244

            band_top = int(height * 0.58)
            band_bottom = int(height * 0.74)
            if band_top <= y <= band_bottom:
                r, g, b = 245, 158, 11

            candle_w = max(2, width // 18)
            candle_h = max(8, height // 5)
            cx0 = width // 2 - candle_w // 2
            cy0 = int(height * 0.32)
            if cx0 <= x < cx0 + candle_w and cy0 <= y < cy0 + candle_h:
                r, g, b = 248, 250, 252

            fx = width // 2
            fy = int(height * 0.24)
            flame_r = max(3, width // 20)
            if (x - fx) ** 2 + (y - fy) ** 2 <= flame_r ** 2:
                r, g, b = 250, 204, 21

            pixels[i : i + 4] = bytes((r, g, b, 255))

    return pixels


def make_screenshot(width: int, height: int, mobile: bool = True) -> bytearray:
    pixels = bytearray(width * height * 4)
    for y in range(height):
        for x in range(width):
            i = (y * width + x) * 4
            r, g, b = 245, 245, 244

            if y < int(height * 0.11):
                r, g, b = 255, 255, 255

            if int(height * 0.05) < y < int(height * 0.09) and x % 90 < 65:
                r, g, b = 231, 229, 228

            if mobile:
                card_w = int(width * 0.84)
                left = (width - card_w) // 2
                for card in range(3):
                    top = int(height * (0.17 + card * 0.22))
                    bottom = top + int(height * 0.16)
                    if left <= x < left + card_w and top <= y < bottom:
                        r, g, b = 255, 255, 255
            else:
                cols = 3
                gap = 18
                card_w = (width - (cols + 1) * gap) // cols
                card_h = int(height * 0.35)
                for col in range(cols):
                    left = gap + col * (card_w + gap)
                    top = int(height * 0.23)
                    if left <= x < left + card_w and top <= y < top + card_h:
                        r, g, b = 255, 255, 255

            pixels[i : i + 4] = bytes((r, g, b, 255))

    return pixels


if __name__ == "__main__":
    icon_specs = [
        ("icon-192.png", 192, 192, False),
        ("icon-512.png", 512, 512, False),
        ("icon-maskable-192.png", 192, 192, True),
        ("icon-maskable-512.png", 512, 512, True),
        ("icon-180.png", 180, 180, False),
    ]

    for name, width, height, maskable in icon_specs:
        write_png(os.path.join(ROOT, name), width, height, make_icon(width, height, maskable=maskable))

    write_png(os.path.join(ROOT, "screenshot-mobile.png"), 540, 720, make_screenshot(540, 720, mobile=True))
    write_png(os.path.join(ROOT, "screenshot-desktop.png"), 720, 540, make_screenshot(720, 540, mobile=False))
