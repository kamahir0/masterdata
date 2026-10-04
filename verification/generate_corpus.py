"""固定oracle formulaからderivative measurement projectを生成する。"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ORACLE = ROOT / 'fixtures/rewrite-oracle/v1'

def navigation(destination: Path):
    spec = json.loads((ORACLE / 'navigation.json').read_text())['input']
    destination.mkdir(parents=True, exist_ok=True)
    (destination / 'masterdata.toml').write_bytes((ORACLE / spec['config']).read_bytes())
    for source in spec['sources']:
        lines = [f"kind: {source['kind']}", f"table: {source['table']}"]
        if source['kind'] == 'schema':
            lines.append('fields:')
            for col in spec['columns']:
                lines.extend([f"  - key: {col['key']}", f"    name: {col['name']}", f"    type: {col['type']}"])
            lines.extend(['primaryKey:', '  fields: [' + ', '.join(spec['primaryKey']) + ']'])
        if source['recordOffset'] is not None:
            lines.append('records:')
            for row in range(spec['rowsPerRecordSource']):
                identity = source['recordOffset'] + row
                for column, col in enumerate(spec['columns']):
                    prefix = '  - ' if column == 0 else '    '
                    lines.append(f"{prefix}{col['name']}: {identity + column}")
        path = destination / source['path']
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('\n'.join(lines) + '\n', encoding='utf-8')

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    if args.destination.resolve().is_relative_to(ROOT / 'fixtures'):
        parser.error('frozen fixtureを出力先にできない')
    navigation(args.destination)
