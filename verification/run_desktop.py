"""Actual native webview measurement; OS manual interactions are separate evidence."""
import argparse
import hashlib
import json
import math
import platform
import plistlib
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from generate_corpus import navigation

ROOT = Path(__file__).resolve().parents[1]


def check(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused'], 'native window not usable'
    assert report['interaction']['singleEditor'] == 1 and report['interaction']['focusedEditor']
    assert report['interaction']['cancelRestoredGrid'], 'editor cancel changed the source or lost grid focus'
    cases = {}
    superseded = 0
    for sample in report['samples']:
        if sample.get('invalid', '').startswith('obsolete'):
            superseded += 1
            continue
        assert not sample.get('invalid'), sample
        if sample['caseName'] in ['dirtySetup', 'dirtyAway']:
            continue
        assert 'firstAccepted' in sample, sample
        assert sample['selectionPublication'] <= sample['ipcReturn'] <= sample['statePublication'] <= sample['reactCommit'] <= sample['layout'] <= sample['paintOpportunity'] <= sample['firstAccepted'], sample
        assert sample['mountedRows'] <= 64, sample
        for counter in ['projectDiscovery', 'projectEnumeration', 'projectYamlParse', 'projectValidation']:
            assert sample['host']['work'][counter] == 0, (counter, sample)
        cases.setdefault(sample['caseName'], []).append(sample['firstAccepted'] - sample['input'])
    for case in ['revisit', 'sameTable', 'crossTable', 'schema']:
        assert len(cases[case]) == 300, case
    assert len(cases['dirtyRevisit']) == 100 and len(cases['rapid']) == 100
    assert superseded == 300, superseded
    dark, light = report['presentation']
    assert dark['shellBackground'] != light['shellBackground'], 'shell theme did not change'
    assert dark['buttonColor'] != light['buttonColor'], 'standard control theme did not change'
    assert dark['explorerColor'] != light['explorerColor'], 'Explorer theme did not change'
    return {case: {'n': len(values), 'medianMs': sorted(values)[len(values)//2],
                   'p95Ms': sorted(values)[math.ceil(len(values)*.95)-1], 'maxMs': max(values)}
            for case, values in cases.items()}


def check_authoring(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused'], 'native window not usable'
    assert [item['name'] for item in report['checks']] == [
        'scalar-composition', 'pointer-range', 'row-drag-cancel', 'row-drag-drop',
        'column-drag-drop', 'array-drag-drop', 'complex-typing-add', 'complex-focus-intent', 'nested-problem-focus', 'complex-navigation',
    ], report['checks']
    assert report['source']['before'] == report['source']['after']
    assert report['schema']['before'] == report['schema']['after']
    assert report['mountedRows'] <= 64
    assert not report['startup'].get('browserErrors')
    return {'checks': len(report['checks']), 'exactSourceAndSchemaRestored': True}


def check_external(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['clean-auto-refresh', 'unrelated-change-preserves-input', 'dirty-conflict',
                                'changed-context-holds-input', 'invalid-and-repair-preserve-draft',
                                'missing-dependency-refresh', 'native-close-cancel-preserves-input']
    assert report['dirty'] == ['sources/catalog-data.yaml'] and report['conflict']
    assert report['mountedRows'] <= 64
    return {'checks': len(report['checks']), 'automaticFilesystemObservation': True}

def check_creation(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['inline-cancel-composition', 'inline-table-exclusive-create',
                               'empty-folder-selection', 'explicit-data-binding-yml',
                               'advanced-lossless-member', 'dirty-revisit-after-creation']
    assert report['dirty'] == ['sources/catalog-data.yaml']
    assert not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'singleArtifactCreation': True}

def check_path(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['path-cancel-composition', 'target-dirty-cancel', 'target-only-save-rename',
                                'case-only-rename', 'target-only-discard-move', 'destination-conflict-no-overwrite']
    assert report['dirty'] == ['sources/catalog-schema.yaml']
    assert report['destination'] == 'sources/moved/Renamed-data.yml'
    assert not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'scopedGuardAndExactMove': True}


def run(binary: Path, output: Path, case: str):
    with tempfile.TemporaryDirectory(prefix='masterdata-desktop-') as work:
        temporary = Path(work)
        project = temporary / 'project'
        if case == 'navigation':
            navigation(project)
        else:
            shutil.copytree(ROOT / 'fixtures/full', project)
            data = project / 'sources/catalog-data.yaml'
            data.write_text(data.read_text().replace('numbers: [1, -2]', 'numbers: [1, "1", -2]'), encoding='utf-8')
            if case == 'external':
                (project / 'sources/unrelated-data.yaml').write_text('kind: data\ntable: item\nrecords: []\n', encoding='utf-8')
            if case == 'path':
                (project / 'sources/moved').mkdir()
        initial_sources = {path.relative_to(project): path.read_bytes() for path in project.rglob('*.yaml')}
        executable = binary.resolve()
        if platform.system() == 'Darwin':
            app = temporary / 'MasterData Rewrite.app' / 'Contents'
            (app / 'MacOS').mkdir(parents=True)
            shutil.copy2(executable, app / 'MacOS' / 'masterdata-desktop')
            (app / 'Info.plist').write_bytes(plistlib.dumps({
                'CFBundleIdentifier': 'dev.masterdata.desktop', 'CFBundleName': 'MasterData Rewrite',
                'CFBundleExecutable': 'masterdata-desktop', 'CFBundlePackageType': 'APPL',
                'NSHighResolutionCapable': True,
            }))
            executable = app / 'MacOS' / 'masterdata-desktop'
        report_path = temporary / 'measurement.json'
        handled = set()
        def external_phase(phase):
            # Only these fixture mutations are allowed; WebView content cannot
            # request arbitrary filesystem writes through the evidence runner.
            data = project / 'sources/catalog-data.yaml'
            schema = project / 'sources/catalog-schema.yaml'
            if phase == 'clean':
                data.write_bytes(initial_sources[Path('sources/catalog-data.yaml')].replace(b'name: Debug Sword', b'name: Outside clean'))
            elif phase == 'unrelated':
                file = project / 'sources/unrelated-data.yaml'
                file.write_bytes(file.read_bytes() + b'\n# External unrelated source\n')
            elif phase == 'dirty':
                data.write_bytes(initial_sources[Path('sources/catalog-data.yaml')].replace(b'name: Debug Sword', b'name: Outside dirty'))
            elif phase == 'invalid': data.write_bytes(b'kind: [\n')
            elif phase == 'restore':
                data.write_bytes(initial_sources[Path('sources/catalog-data.yaml')].replace(b'name: Debug Sword', b'name: Outside restored'))
            elif phase == 'delete-schema': schema.unlink()
            elif phase == 'restore-schema': schema.write_bytes(initial_sources[Path('sources/catalog-schema.yaml')])
            else: raise AssertionError(f'unknown external evidence phase {phase}')
        with (output.parent / 'desktop-process.log').open('w') as log:
            process = subprocess.Popen([str(executable), '--project', str(project),
                                        '--evidence-output', str(report_path), '--evidence-kind', case], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 240
                while True:
                    assert process.poll() is None, f'native host exited: {process.returncode}'
                    if time.monotonic() >= deadline:
                        raise RuntimeError('native Desktop evidence unavailable: visible focused window did not complete')
                    if report_path.exists():
                        try: report = json.loads(report_path.read_text())
                        except json.JSONDecodeError: report = None
                        if report is not None:
                            if case == 'external' and report.get('phase'):
                                phase = report['phase']
                                if phase not in handled:
                                    external_phase(phase)
                                    handled.add(phase)
                            else: break
                    time.sleep(.1)
                report['environment'] = {'os': platform.system(), 'release': platform.release(),
                                         'architecture': platform.machine(),
                                         'kind': 'actual native WebView; controlled DOM keyboard/pointer handlers; OS input is separate'}
                report['implementation'] = {
                    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                    'workingTreeChanged': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True)),
                    'binarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                }
                # A failed assertion must retain the measurements that caused it.
                output.write_text(json.dumps(report, indent=2))
                report['summary'] = {'navigation': check, 'external': check_external, 'creation': check_creation, 'authoring': check_authoring, 'path': check_path}[case](report)
                if case == 'external':
                    expected = initial_sources[Path('sources/catalog-data.yaml')].replace(b'name: Debug Sword', b'name: Outside restored')
                    assert (project / 'sources/catalog-data.yaml').read_bytes() == expected, 'local draft silently overwrote external source'
                elif case == 'path':
                    original = Path('sources/catalog-data.yaml')
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items() if path != original), 'Move saved unrelated source'
                    assert not (project / original).exists()
                    assert (project / report['destination']).read_bytes() == initial_sources[original].replace(b'name: Debug Sword', b'name: Saved before move'), 'Move rewrote source or saved discarded draft'
                    assert not (project / 'sources/cancelled.yaml').exists()
                    assert not (project / '.masterdata').exists(), 'Move implicitly built artifacts'
                else:
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items()), 'interaction implicitly wrote source'
                if case == 'creation':
                    new_sources = {str(path.relative_to(project)).replace('\\', '/') for pattern in ('*.yaml', '*.yml') for path in project.rglob(pattern)} - {str(path).replace('\\', '/') for path in initial_sources}
                    assert new_sources == set(report['created']), 'implicit / missing created source'
                    assert not (project / 'sources/cancelled.yaml').exists()
                    assert 'table: fresh-table' in (project / 'sources/fresh-table.yaml').read_text()
                    assert 'table: fresh-table' in (project / 'sources/catalog-new/storage-name.yml').read_text()
                    assert 'value: 18446744073709551615' in (project / 'sources/catalog-new/huge-token.yaml').read_text()
                    assert not (project / 'artifacts').exists(), 'creation implicitly built artifacts'
                output.write_text(json.dumps(report, indent=2))
                print(json.dumps(report['summary'], indent=2))
            finally:
                process.terminate()
                process.wait(timeout=10)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--case', choices=['navigation', 'authoring', 'external', 'creation', 'path'], default='navigation')
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    run(args.binary, args.output, args.case)
