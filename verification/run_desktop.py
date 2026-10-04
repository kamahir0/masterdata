"""Actual native webview measurement; OS manual interactions are separate evidence."""
import argparse
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


def run(binary: Path, output: Path):
    with tempfile.TemporaryDirectory(prefix='masterdata-desktop-') as work:
        temporary = Path(work)
        project = temporary / 'project'
        navigation(project)
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
        with (output.parent / 'desktop-process.log').open('w') as log:
            process = subprocess.Popen([str(executable), '--project', str(project),
                                        '--evidence-output', str(report_path)], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 240
                while not report_path.exists():
                    assert process.poll() is None, f'native host exited: {process.returncode}'
                    if time.monotonic() >= deadline:
                        raise RuntimeError('native Desktop evidence unavailable: visible focused window did not complete')
                    time.sleep(.1)
                report = json.loads(report_path.read_text())
                report['environment'] = {'os': platform.system(), 'release': platform.release(),
                                         'architecture': platform.machine(), 'kind': 'actual native webview; controlled DOM keyboard handler'}
                # A failed assertion must retain the measurements that caused it.
                output.write_text(json.dumps(report, indent=2))
                report['summary'] = check(report)
                output.write_text(json.dumps(report, indent=2))
                print(json.dumps(report['summary'], indent=2))
            finally:
                process.terminate()
                process.wait(timeout=10)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    run(args.binary, args.output)
