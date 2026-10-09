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


def capacity(destination: Path):
    spec = json.loads((ORACLE / 'capacity.json').read_text())['input']
    assert spec['recordId'] == 'fileIndex * 10000 + rowIndex'
    assert spec['fieldValue'] == 'recordId + columnIndex'
    destination.mkdir(parents=True, exist_ok=True)
    (destination / 'masterdata.toml').write_text('[project]\nid = "oracle.capacity"\nname = "Capacity Oracle"\nversion = "0.1.0"\n\n[sources]\nroots = ["sources"]\n\n[build]\nartifact_dir = ".masterdata/output"\ncache = ".masterdata/cache"\n', encoding='utf-8', newline='\n')
    directory = destination / 'sources'
    directory.mkdir(exist_ok=True)
    fields = [f'field{column:02}' for column in range(spec['columns'])]
    schema = ['kind: schema', 'table: capacity', 'fields:']
    for column, field in enumerate(fields):
        schema.extend([f'  - key: {column}', f'    name: {field}', '    type: int'])
    schema.extend(['primaryKey:', '  fields: [field00]'])
    (directory / 'schema.yaml').write_text('\n'.join(schema) + '\n', encoding='utf-8', newline='\n')
    for file in range(spec['dataFiles']):
        lines = ['kind: data', 'table: capacity', 'records:']
        for row in range(spec['recordsPerFile']):
            identity = file * spec['recordsPerFile'] + row
            for column, field in enumerate(fields):
                lines.append(f'{"  - " if column == 0 else "    "}{field}: {identity + column}')
        (directory / f'data{file:02}.yaml').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('destination', type=Path)
    parser.add_argument('--case', choices=['navigation', 'capacity'], default='navigation')
    args = parser.parse_args()
    if args.destination.resolve().is_relative_to(ROOT / 'fixtures'):
        parser.error('frozen fixtureを出力先にできない')
    {'navigation': navigation, 'capacity': capacity}[args.case](args.destination)
