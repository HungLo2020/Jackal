#!/usr/bin/env python3
"""Capture isolated original boss actor sequences; normal builds never run this."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from export_graphics import ROOT,HERE,REFERENCE,FILES
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
with tempfile.TemporaryDirectory(prefix='jackal-boss-reference-') as temp:
    root=Path(temp)
    for name in FILES:
        target=root/name;target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(subprocess.check_output(['git','show',f'{REFERENCE}:{name}'],cwd=ROOT))
    manifest=root/'Cargo.toml'
    manifest.write_text(manifest.read_text().replace('rodio = "0.20.1"','rodio = "0.20.1"\nserde_json = "1.0"'))
    with (root/'src/game.rs').open('a') as stream:stream.write((HERE/'enemy_sequences_methods.rs').read_text())
    (root/'src/main.rs').write_text((HERE/'boss_sequences_main.rs').read_text())
    raw=subprocess.check_output(['cargo','run','--offline','--manifest-path',str(manifest)],cwd=root,env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target/reference-bosses')))
    captures=json.loads(raw)
    groups={'boss_sequences.json':[24,146,49,64], 'boss_vehicle_sequences.json':[10,35,36], 'hq_sequences.json':[74,71,75,79], 'hq_children.json':[72,76,77]}
    for name,types in groups.items():
        result=json.dumps([c for c in captures if c['source_type'] in types],separators=(',',':'))+'\n'
        output=ROOT/'tests/fixtures'/name
        if args.check:
            if result!=output.read_text():raise SystemExit(f'{name} reference capture differs')
        else:output.write_text(result)
    pod=next(c for c in captures if c['source_type']==66)
    pod['samples']=pod['samples'][:105]
    output=ROOT/'tests/fixtures/paratrooper_pod.json'
    result=json.dumps(pod,separators=(',',':'))+'\n'
    if args.check:
        if json.loads(output.read_text())!=pod:raise SystemExit('Paratrooper reference capture differs')
        print('Boss reference captures verified')
    else:output.write_text(result)

    (root/'src/main.rs').write_text((HERE/'boss_deaths_main.rs').read_text())
    raw=subprocess.check_output(['cargo','run','--offline','--manifest-path',str(manifest)],cwd=root,env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target/reference-bosses')))
    result=json.dumps(json.loads(raw),separators=(',',':'))+'\n'
    output=ROOT/'tests/fixtures/boss_deaths.json'
    if args.check:
        if result!=output.read_text():raise SystemExit('Boss death reference capture differs')
        print('Boss death reference capture verified')
    else:output.write_text(result)
