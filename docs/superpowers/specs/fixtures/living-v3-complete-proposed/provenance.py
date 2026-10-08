"""Read-only exact authority checks; never refresh pins during verification."""
import hashlib
import json
from pathlib import Path
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
SDD = ROOT / '.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
FREEZE = SDD / 'v3-ia2-completion-lane-freeze-2'
MANIFEST_HASH = '55ca0ede26a284142f6d1208fa57ebe83ecd6bcce59a99751751332d42d6d1eb'

def identity(raw):
    return {'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}

def check_sources(pins_path=None):
    raw = (FREEZE / 'manifest.json').read_bytes()
    if identity(raw)['sha256'] != MANIFEST_HASH:
        raise ValueError('authority manifest drift')
    authority = json.loads(raw)['sources']
    pins = json.loads((pins_path or HERE / 'source_hashes.json').read_bytes())
    if pins != {'authority_manifest_sha256': MANIFEST_HASH, 'sources': authority} or len(authority) != 8:
        raise ValueError('source pin roster drift')
    for name, expected in authority.items():
        for path in (FREEZE / name, ROOT / expected['source_path']):
            if identity(path.read_bytes()) != {k: expected[k] for k in ('bytes', 'sha256')}:
                raise ValueError('source drift: ' + str(path))
    old = json.loads((SDD / 'v3-foundation-old-corpus-protection-before.json').read_bytes())
    for name, expected in old.items():
        if identity((ROOT / name).read_bytes()) != expected:
            raise ValueError('protected old corpus drift: ' + name)
    return authority
