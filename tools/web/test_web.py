from pathlib import Path
p=Path(__file__).parent/'chamber00.html'
assert p.exists(), 'single-file chamber artifact missing'
s=p.read_text()
assert p.stat().st_size < 9_000_000
assert 'WebGL2' in s and 'DecompressionStream' in s
assert 'https://' not in s and 'fetch(' not in s
print('PASS: embedded offline artifact under 9,000,000 bytes')
