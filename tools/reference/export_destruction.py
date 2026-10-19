#!/usr/bin/env python3
"""Recover editable destroyed-building tile patches from the historical reference.

Static evidence: bank 6 BABC/BB05/ADC1 selects the patch; fixed-bank
F33D/F350 applies signed offsets, and DF6E/E03C writes row-major background
characters without replacing palette attributes. Runtime builds do not use this tool.
"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
REFERENCE = 'e8fc3391d5b27fcd98b82fd27984141840449708'


def extract(prg):
    def data(address, size):
        bank, base = (7, 0xc000) if address >= 0xc000 else (6, 0x8000)
        offset = bank * 16384 + address - base
        return prg[offset:offset + size]

    patches = []
    for stage in range(6):
        for source_type in [0x13, 0x14, 0x15, 0x19, 0x1c, 0x1d]:
            if source_type in [0x14, 0x1d]:
                patch = data(0xbae5 + stage, 1)[0]
            elif source_type in [0x13, 0x1c]:
                patch = data(0xbaeb + stage, 1)[0]
            elif source_type == 0x15:
                patch = data(0xbb18 + stage, 1)[0]
            else:
                patch = 16
            address = int.from_bytes(data(0xe099 + patch * 2, 2), 'little')
            patches.append(dict(stage=stage + 1, source_type=source_type,
                offset_x=int.from_bytes(data(0xf394 + patch, 1), 'little', signed=True),
                offset_y=int.from_bytes(data(0xf3b1 + patch, 1), 'little', signed=True),
                width=4, height=4, collision=data(address+16,1)[0], tiles=list(data(address, 16))))
    return dict(description='Original permanent rubble tiles; retain existing palette attributes.', patches=patches)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    prg = subprocess.check_output(['git', 'show', f'{REFERENCE}:assets/game-data.bin'], cwd=ROOT)
    output = json.dumps(extract(prg), indent=2) + '\n'
    path = ROOT / 'assets/destruction.json'
    if args.check:
        if path.read_text() != output:
            raise SystemExit('Destruction definitions differ from original tables')
        print('Original destruction tables verified')
    else:
        path.write_text(output)


if __name__ == '__main__':
    main()
