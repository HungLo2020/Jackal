#!/usr/bin/env python3
"""Export the original HUD glyphs from the already-decoded native sprite atlas."""
from pathlib import Path
from PIL import Image
ROOT = Path(__file__).resolve().parents[1]
source = Image.open(ROOT / 'assets/textures/stage_01_sprites.png').convert('RGBA')
font = Image.new('RGBA', (96, 16))
for cell, (tile, palette) in enumerate([(2 + 2 * digit, 0) for digit in range(10)] + [(32, 0), (32, 1)]):
    x, y = tile % 16 * 8, (palette * 16 + tile // 16) * 16
    font.paste(source.crop((x, y, x + 8, y + 16)), (cell * 8, 0))
(ROOT / 'assets/fonts').mkdir(exist_ok=True)
font.save(ROOT / 'assets/fonts/hud.png')
