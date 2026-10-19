#!/usr/bin/env python3
"""Capture isolated original enemy handler sequences; normal builds never run this."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from export_graphics import ROOT,HERE,REFERENCE,FILES
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
with tempfile.TemporaryDirectory(prefix='jackal-enemy-reference-') as temp:
    root=Path(temp)
    for name in FILES:
        target=root/name;target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(subprocess.check_output(['git','show',f'{REFERENCE}:{name}'],cwd=ROOT))
    manifest=root/'Cargo.toml'
    manifest.write_text(manifest.read_text().replace('rodio = "0.20.1"','rodio = "0.20.1"\nserde_json = "1.0"'))
    with (root/'src/game.rs').open('a') as stream:stream.write((HERE/'enemy_sequences_methods.rs').read_text())
    (root/'src/main.rs').write_text((HERE/'prisoner_sequences_main.rs').read_text())
    raw=subprocess.check_output(['cargo','run','--offline','--manifest-path',str(manifest)],cwd=root,env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target/reference-enemies')))
    captures=json.loads(raw)
    result=json.dumps(captures,separators=(',',':'))+'\n'
    output=ROOT/'tests/fixtures/prisoners.json'
    if args.check:
        if result!=output.read_text():raise SystemExit('Enemy reference capture differs')
        print('Enemy reference capture verified')
    else:output.write_text(result)
