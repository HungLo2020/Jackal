#!/usr/bin/env python3
"""Reproduce release art from the documented cover scan and native jeep atlas.

Requires Pillow only when regenerating artwork; release builds use committed files.
"""
import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[2]
RESAMPLING = getattr(Image, "Resampling", Image)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Downloaded original LaunchBox JPEG")
    args = parser.parse_args()
    destination = ROOT / "packaging/artwork"
    source = Image.open(args.source).convert("RGB")
    cover = ImageOps.fit(source, (900, 1200), method=RESAMPLING.LANCZOS)
    cover.save(destination / "cover.jpg", quality=90, optimize=True, progressive=True)

    atlas = Image.open(ROOT / "assets/textures/stage_01_sprites.png").convert("RGBA")
    frame = json.loads((ROOT / "assets/levels/sprites.json").read_text())["frames"][1]
    jeep = Image.new("RGBA", (16, 32))
    for part in reversed(frame["parts"]):
        index = part["tile"] + part["palette"] * 256
        x, y = index % (atlas.width // 8) * 8, index // (atlas.width // 8) * 16
        tile = atlas.crop((x, y, x + 8, y + 16))
        if part["flip_x"]:
            tile = ImageOps.mirror(tile)
        if part["flip_y"]:
            tile = ImageOps.flip(tile)
        jeep.alpha_composite(tile, (part["x"] + 8, part["y"] + 16))
    icon = Image.new("RGBA", (512, 512))
    icon.alpha_composite(jeep.resize((192, 384), RESAMPLING.NEAREST), (160, 64))
    icon.save(destination / "icon.png", optimize=True)
    provenance = {
        "cover_page": "https://gamesdb.launchbox-app.com/games/images/336-jackal",
        "cover_original": "https://images.launchbox-app.com/93ca17a0-65dd-4bde-bf73-1404c68c8c76.jpg",
        "source_sha256": hashlib.sha256(args.source.read_bytes()).hexdigest(),
        "source_dimensions": list(source.size),
        "cover_transform": "Center crop to 3:4; Lanczos resize to 900x1200; RGB JPEG quality 90",
        "icon_source": "assets/textures/stage_01_sprites.png, sprites.json frame 1",
        "icon_transform": "Composite original jeep; nearest-neighbor scale; transparent 512x512 canvas",
        "credit": "Original Jackal game and artwork: Konami. Cover scan hosted by LaunchBox Games Database.",
    }
    (destination / "sources.json").write_text(json.dumps(provenance, indent=2) + "\n")


if __name__ == "__main__":
    main()
