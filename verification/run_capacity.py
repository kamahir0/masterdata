"""Fixed capacity oracle with native process peak memory; Desktop is separate."""
import argparse
import hashlib
import json
import platform
import subprocess
import tempfile
import time
from pathlib import Path

from generate_corpus import capacity
from process_memory import child_peak, windows_peak

ROOT = Path(__file__).resolve().parents[1]


def run(binary, output):
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='masterdata-capacity-') as work:
        project = Path(work) / 'project'
        capacity(project)
        stdout = Path(work) / 'native.json'
        log = output.with_suffix('.log')
        peak = 0
        started = time.monotonic()
        with stdout.open('w') as result, log.open('w') as diagnostics:
            process = subprocess.Popen([str(binary.resolve()), str(project)], stdout=result, stderr=diagnostics, stdin=subprocess.DEVNULL)
            deadline = started + 1200
            try:
                while process.poll() is None:
                    if platform.system() == 'Windows':
                        peak = max(peak, windows_peak(process))
                    if time.monotonic() >= deadline:
                        raise RuntimeError('capacity process did not complete')
                    time.sleep(.1)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait()
        assert process.returncode == 0, f'capacity failed ({process.returncode}); see {log}'
        peak, boundary = child_peak(peak)
        report = json.loads(stdout.read_text(encoding='utf-8'))
        report['peakRssBytes'] = peak
        report['peakRssBoundary'] = boundary
        report['environment'] = {'os': platform.system(), 'release': platform.release(), 'architecture': platform.machine(), 'boundary': 'native engine; Desktop rendering not measured'}
        report['implementation'] = {
            'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
            'workingTreeChanged': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True)),
            'binarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
            'oracleSha256': hashlib.sha256((ROOT / 'fixtures/rewrite-oracle/v1/capacity.json').read_bytes()).hexdigest(),
        }
        report['wallMs'] = (time.monotonic() - started) * 1000
        output.write_text(json.dumps(report, indent=2), encoding='utf-8')
        assert report['checkedCellsBefore'] == report['checkedCellsAfter'] == 2_000_000
        assert report['pasteChangedCells'] == 10_000 and report['exactCandidateBytes'] and report['diskUnchanged']
        assert report['diagnostics'] == [] and peak > 0
        print(json.dumps({'records': report['records'], 'checkedCells': report['checkedCellsAfter'], 'peakRssMiB': peak/1024**2, 'stageMs': {key: value['elapsedMs'] for key, value in report['stages'].items()}}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT / 'verification/results/capacity-native.json')
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(ROOT / 'fixtures'):
        parser.error('frozen fixture cannot be output')
    run(args.binary, args.output)
