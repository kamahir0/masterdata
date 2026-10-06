"""Inspect trusted native input evidence without calling tool roundtrip latency usability."""
import argparse
import json
import math
import statistics
from collections import Counter
from pathlib import Path

WARM = ('revisit', 'sameTable', 'crossTable', 'schema', 'dirtyRevisit', 'rapid')
COUNTERS = ('projectDiscovery', 'projectEnumeration', 'projectYamlParse', 'projectValidation')
STAGES = ('input', 'selectionPublication', 'ipcReturn', 'statePublication', 'reactCommit', 'layout', 'paintOpportunity')


def distribution(values, unit='Ms'):
    values = sorted(values)
    if len(values) == 1:
        return {'n': 1, 'single' + unit: values[0]}
    return {'n': len(values), 'median' + unit: statistics.median(values),
            'p95' + unit: values[math.ceil(len(values) * .95) - 1], 'max' + unit: values[-1]} if values else {'n': 0}


def analyze(reports, max_probe_gap):
    cases = {}
    runs = []
    for path in reports:
        report = json.loads(path.read_text(encoding='utf-8'))
        assert report['kind'] == 'actual-os-input', 'controlled DOM report is not OS input evidence'
        metadata_path = path.parent / 'metadata.json'
        metadata = json.loads(metadata_path.read_text(encoding='utf-8')) if metadata_path.exists() else None
        runs.append({'file': str(path), 'metadata': metadata, 'startup': report['startup'],
                     'current': report['current'], 'droppedSamples': report['droppedSamples']})
        assert not report['startup'].get('browserErrors'), 'browser error during actual input capture'
        for sample in report['samples']:
            case = cases.setdefault(sample['caseName'], {'samples': [], 'invalid': Counter(), 'superseded': 0, 'programmatic': 0, 'runs': set()})
            if sample.get('inputOrigin') != 'os-trusted':
                case['programmatic'] += 1
                continue
            if sample.get('invalid', '').startswith('obsolete'):
                case['superseded'] += 1
                continue
            if sample.get('invalid'):
                case['invalid'][sample['invalid']] += 1
                continue
            missing = [key for key in STAGES if key not in sample]
            if missing:
                case['invalid']['missing boundaries: ' + ', '.join(missing)] += 1
                continue
            times = [sample[key] for key in STAGES]
            assert times == sorted(times), ('out-of-order boundaries', sample)
            for boundary in ('input', 'paint'):
                assert sample['observations'][boundary] == {'visibility': 'visible', 'focused': True}, (boundary, sample)
            assert sample['mountedRows'] <= 64, ('unbounded viewport', sample)
            assert all(sample['host']['work'][key] == 0 for key in COUNTERS), ('warm project-wide work', sample)
            probes = [p['at'] for p in sample.get('probes', []) if p['origin'] == 'os-trusted' and p['target'] == sample['target']]
            accepted = sample.get('firstAccepted')
            if accepted is not None:
                assert sample.get('firstAcceptedOrigin') == 'os-trusted', ('fake first interaction', sample)
                assert accepted >= times[-1]
                assert sample['observations']['interaction'] == {'visibility': 'visible', 'focused': True}
            # A late first probe gives an observed upper bound. It cannot prove
            # failure of the editor target or be silently replaced by rAF time.
            previous = sample['paintOpportunity']
            gaps = []
            for at in probes:
                if at >= previous and (accepted is None or at <= accepted):
                    gaps.append(at - previous)
                    previous = at
            if accepted is not None:
                gaps.append(accepted - previous)
            timely = accepted is not None and bool(probes) and max(gaps, default=math.inf) <= max_probe_gap
            case['samples'].append((sample, timely, max(gaps, default=None)))
            case['runs'].add(str(path))
    summaries = {}
    for name, case in cases.items():
        samples = case['samples']
        metrics = {'selectionFeedback': [], 'eventToSelectionFeedback': [], 'eventDelivery': [], 'backend': [], 'serialization': [], 'queue': [], 'bytes': [],
                   'stateAfterIpc': [], 'reactCommit': [], 'layout': [], 'paintOpportunity': [],
                   'eventToPaintOpportunity': [], 'firstAcceptedObservedUpperBound': [],
                   'eventToFirstAcceptedObservedUpperBound': [], 'firstAcceptedTimelyProbe': [], 'probeGap': []}
        no_probe = 0
        event_clock_missing = 0
        for sample, timely, gap in samples:
            # DOM Event.timeStamp and performance.now share the Window's time
            # origin. Keep event creation → handler queueing visible; measuring
            # only from the handler can hide a busy UI thread. Older/unavailable
            # event clocks are explicitly excluded, never guessed from OS wall.
            # https://developer.mozilla.org/en-US/docs/Web/API/Event/timeStamp
            timestamp = sample.get('eventTimestamp')
            event_clock = isinstance(timestamp, (int, float)) and math.isfinite(timestamp) and 0 < timestamp <= sample['input'] + 1
            if event_clock:
                metrics['eventDelivery'].append(sample['input'] - timestamp)
                metrics['eventToSelectionFeedback'].append(sample['selectionPublication'] - timestamp)
                metrics['eventToPaintOpportunity'].append(sample['paintOpportunity'] - timestamp)
            else:
                event_clock_missing += 1
            metrics['selectionFeedback'].append(sample['selectionPublication'] - sample['input'])
            metrics['backend'].append(sample['host']['backendMs'])
            metrics['serialization'].append(sample['host']['serializationMs'])
            metrics['queue'].append(sample['host']['queuedMs'])
            metrics['bytes'].append(sample['host']['bytes'])
            metrics['stateAfterIpc'].append(sample['statePublication'] - sample['ipcReturn'])
            metrics['reactCommit'].append(sample['reactCommit'] - sample['statePublication'])
            metrics['layout'].append(sample['layout'] - sample['reactCommit'])
            metrics['paintOpportunity'].append(sample['paintOpportunity'] - sample['input'])
            if sample.get('firstAccepted') is None:
                no_probe += 1
            else:
                latency = sample['firstAccepted'] - sample['input']
                metrics['firstAcceptedObservedUpperBound'].append(latency)
                if event_clock:
                    event_latency = sample['firstAccepted'] - timestamp
                    metrics['eventToFirstAcceptedObservedUpperBound'].append(event_latency)
                    if timely:
                        metrics['firstAcceptedTimelyProbe'].append(event_latency)
            if gap is not None:
                metrics['probeGap'].append(gap)
        summaries[name] = {'n': len(samples), 'runs': len(case['runs']), 'invalid': dict(case['invalid']),
                           'superseded': case['superseded'], 'programmatic': case['programmatic'], 'missingFirstInteraction': no_probe,
                           'missingEventClock': event_clock_missing,
                           'workCounts': {key: 0 for key in COUNTERS},
                           'metrics': {key: distribution(values, 'Bytes' if key == 'bytes' else 'Ms') for key, values in metrics.items()}}
    complete = all(case in summaries and summaries[case]['metrics']['firstAcceptedTimelyProbe']['n'] >= 100
                   and summaries[case]['runs'] >= 2 for case in WARM)
    return {'format': 1, 'measurement': 'actual OS input; event creation / handler start / first acceptance are separate; rAF is paint opportunity, not GPU presentation',
            'probeMaximumGapMs': max_probe_gap, 'warmDistributionComplete': complete,
            'finalCandidateAttested': bool(runs) and all(run['metadata'] and run['metadata'].get('finalCandidate')
                and not run['metadata'].get('workingTreeChanged', True) for run in runs)
                and len({(run['metadata']['head'], run['metadata']['binarySha256']) for run in runs if run['metadata']}) == 1,
            'runs': runs, 'cases': summaries}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('reports', nargs='+', type=Path)
    parser.add_argument('--max-probe-gap-ms', type=float, default=33)
    parser.add_argument('--require-warm-distribution', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    summary = analyze(args.reports, args.max_probe_gap_ms)
    encoded = json.dumps(summary, ensure_ascii=False, indent=2)
    if args.output:
        root = Path(__file__).resolve().parents[1]
        if args.output.resolve().is_relative_to(root / 'fixtures'):
            parser.error('frozen oracle cannot be output')
        args.output.write_text(encoded + '\n', encoding='utf-8')
    else:
        print(encoded)
    if args.require_warm_distribution and not summary['warmDistributionComplete']:
        parser.exit(1, 'actual OS-input warm distributions / timely probes are incomplete\n')
