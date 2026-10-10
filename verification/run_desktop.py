"""Actual native webview measurement; OS manual interactions are separate evidence."""
import argparse
import hashlib
import json
import platform
import plistlib
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

from generate_corpus import capacity, navigation
from process_memory import child_peak, windows_peak
from analyze_os_input import distribution

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
        for boundary in ['input', 'paint', 'interaction']:
            assert sample['observations'][boundary] == {'visibility': 'visible', 'focused': True}, (boundary, sample)
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
    return {case: distribution(values) for case, values in cases.items()}


def check_authoring(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused'], 'native window not usable'
    assert [item['name'] for item in report['checks']] == [
        'scalar-composition', 'pointer-range', 'row-drag-cancel', 'row-drag-drop',
        'bidirectional-row-motion', 'between-frame-row-drop', 'column-drag-drop', 'bidirectional-column-motion', 'array-drag-drop', 'complex-typing-add', 'complex-focus-intent', 'nested-problem-focus', 'complex-navigation',
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
                                'compare-isolates-physical-source',
                                'obsolete-compare-cannot-replace-or-reopen',
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


def check_migration(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['inline-rename-composition-cancel', 'failed-rename-preserves-input',
                                'direct-rename-exact-occurrences', 'tail-add-explicit-null-focus',
                                'relative-column-insert', 'drop-compare-explicit-authorization']
    assert not report['startup']['browserErrors']
    assert report['dirty'] == []
    return {'checks': len(report['checks']), 'directStructuralAuthoring': True}


def check_type(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['resolved-type-selection-warm-zero', 'type-keyboard-menu-cancel-focus', 'value-object-plan-diff-apply',
                               'flags-none-protected', 'external-stale-plan-explicit-replan', 'enum-ulong-lossless-add',
                               'enum-rename-preserves-number', 'unused-member-authorized-drop',
                               'custom-long-min-guided-initializer', 'custom-rename-occurrences',
                               'custom-authorized-drop', 'affected-dirty-plan-apply-gate', 'unrelated-draft-history-survives-apply']
    assert report['dirty'] == ['sources/catalog-data.yaml', 'sources/independent.yaml']
    assert not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'typedOperationsAndExplicitApply': True}


def check_delivery(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['contextual-drawer-saved-input', 'build-live-navigation-draft-and-warm-zero',
                                'native-build-then-preview-cancel-retains-artifacts', 'missing-profile-publish-independent',
                                'stale-preview-fresh-confirm-no-write', 'late-completion-preserves-newer-focus',
                                'confirmed-receipt-publish-invalid-source']
    assert report['state']['result']['unityVerification'] == 'not_observed'
    assert report['dirty'] == ['sources/catalog-data.yaml']
    assert not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'nativeBuildAndReceiptOnlyPublish': True}


def check_capacity(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['long-wide-bounded-projection', 'find-exact-empty-without-dirty',
                                'ordinary-paste-exact-visible-targets', 'paste-one-undo-restores-clean-source',
                                'redo-restores-paste-candidate']
    assert report['bounds']['mountedRows'] <= 64 and report['bounds']['mountedCells'] <= 64 * 20
    assert report['bounds']['activeEditors'] == 1 and not report['startup']['browserErrors']
    assert report['dirty'] == ['sources/data00.yaml'] and report['diagnostics'] == []
    samples = [sample for sample in report['samples'] if sample['caseName'] == 'capacityWarm']
    assert len(samples) == 100
    for sample in samples:
        assert not sample.get('invalid'), sample
        for boundary in ['input', 'paint', 'interaction']:
            assert sample['observations'][boundary] == {'visibility': 'visible', 'focused': True}, (boundary, sample)
        assert sample['selectionPublication'] <= sample['ipcReturn'] <= sample['statePublication'] <= sample['reactCommit'] <= sample['layout'] <= sample['paintOpportunity'] <= sample['firstAccepted'], sample
        assert sample['mountedRows'] <= 64
        for counter in ['projectDiscovery', 'projectEnumeration', 'projectYamlParse', 'projectValidation']:
            assert sample['host']['work'][counter] == 0, (counter, sample)
    values = sorted(sample['firstAccepted'] - sample['input'] for sample in samples)
    return {'checks': len(report['checks']), 'warm': distribution(values), 'boundedRendering': True}


def check_tags(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['keyboard-context-open-clean-known-tags', 'composition-and-lossless-invalid-tag',
                                'close-retains-one-undo-redo', 'replace-duplicate-and-explicit-empty',
                                'problems-resolves-tag-editor', 'added-row-tags-delete-composition']
    assert report['source']['before'] == report['source']['after']
    assert report['mountedRows'] <= 64 and not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'exactTagHistoryAndNoImplicitWrite': True}


def check_settings(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['config-open-clean-yaml-independent', 'profile-compose-explicit-identity',
                                'settings-active-save-domain-invalid', 'section-target-draft-navigation',
                                'config-conflict-compare-reload', 'config-unknown-reply-recheck', 'all-dirty-guard-cancel-discard']
    assert report['source']['before'] == report['source']['after']
    assert report['config']['before'] == report['config']['after']
    assert report['mountedRows'] <= 64 and not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'configSaveAndIndependentYamlDraft': True}

def check_project(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['welcome-recent-and-appearance-entry', 'welcome-create-cancel-focus', 'welcome-recents-explicit-open-remove', 'unknown-open-reply-explicit-reopen',
                               'sidebar-whole-pane-state-preserved', 'source-only-tree-contextual-find-cancel', 'contextual-find-type-table-warm-zero', 'create-all-dirty-guard-cancel',
                               'create-form-cancel-composition', 'nonempty-failure-retains-workspace',
                               'unknown-creation-reply-explicit-open', 'guided-schema-explicit-no-record-source',
                               'explicit-data-logical-navigation-shared-draft', 'recent-canonical-root-order-dedup']
    assert not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'explicitProjectAndArtifactLifecycles': True}


def check_declaration(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['open-close-physical-draft-focus', 'messagepack-review-compare-composition',
        'primary-key-direct-ordered-components', 'secondary-key-composite-order', 'reference-add-rust-resolved-feedback',
        'reference-helper-override', 'incoming-reference-key-removal-rejected', 'reference-remove-explicit-authorization',
        'stale-plan-no-auto-replan-apply', 'unknown-reply-explicit-recheck', 'dirty-revisit-undo-warm-zero-stable-grid']
    assert not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'exactPhysicalSchemaAndPreservedRecordHistory': True}


def check_focus(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['programmatic-is-not-os-input', 'synthetic-is-not-os-input',
                               'enter-composition-pane-focus-and-return', 'completion-preserves-newer-pane-focus',
                               'latest-keyboard-open-wins', 'event-trust-expires-after-dispatch']
    assert not report['startup']['browserErrors'] and not report['dirty']
    assert all('firstAccepted' not in sample and sample['inputOrigin'] in ['synthetic', 'programmatic'] for sample in report['samples'])
    return {'checks': len(report['checks']), 'keyboardPaneFocus': True, 'osInputNeverForged': True}


def check_geometry(report):
    assert not report.get('error'), report.get('error')
    assert report['visibility'] == 'visible' and report['focused']
    assert report['checks'] == ['header-name-handle-action-separate', 'sticky-corner-occludes-scrolled-controls',
                               'inline-editor-matches-display-typography', 'sticky-row-context-occludes-active-editor', 'scroll-preserves-temporary-input-and-escape-discards',
                               'row-menu-dismiss-and-focus', 'viewport-fixed-add-actions', 'long-wide-sticky-context-and-exact-value',
                               'contextual-popup-remains-interactive']
    assert report['mountedRows'] <= 64 and not report['dirty'] and not report['startup']['browserErrors']
    return {'checks': len(report['checks']), 'stickyHitAreasProtected': True}


def run(binary: Path, output: Path, case: str):
    with tempfile.TemporaryDirectory(prefix='masterdata-desktop-') as work:
        temporary = Path(work)
        project = temporary / 'project'
        if case in ['navigation', 'capacity', 'geometry']:
            {'navigation': navigation, 'capacity': capacity, 'geometry': navigation}[case](project)
        else:
            shutil.copytree(ROOT / 'fixtures/full', project)
            data = project / 'sources/catalog-data.yaml'
            if case != 'delivery':
                data.write_text(data.read_text(encoding='utf-8').replace('numbers: [1, -2]', 'numbers: [1, "1", -2]'), encoding='utf-8')
            else:
                config = project / 'masterdata.toml'
                config.write_text(config.read_text(encoding='utf-8') + '\n[build.profiles.development]\ninclude_tags = ["development"]\n', encoding='utf-8')
            if case == 'external':
                (project / 'sources/unrelated-data.yaml').write_text('kind: data\ntable: item\nrecords: []\n', encoding='utf-8')
                for file in [data, project / 'sources/catalog-schema.yaml']:
                    file.write_bytes(file.read_bytes().replace(b'\r\n', b'\n').replace(b'\n', b'\r\n'))
            if case == 'path':
                (project / 'sources/moved').mkdir()
            if case == 'type':
                (project / 'sources/ranges.yaml').write_text('kind: type\nname: Range\nenum:\n  underlying: ulong\n  members:\n    - name: Zero\n      value: 0\n', encoding='utf-8')
                (project / 'sources/independent.yaml').write_text('kind: schema\ntable: independent\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n', encoding='utf-8')
            if case == 'declaration':
                schema = project / 'sources/catalog-schema.yaml'
                schema.write_bytes(schema.read_bytes().replace(b'\r\n', b'\n').replace(b'\n', b'\r\n'))
        initial_sources = {path.relative_to(project): path.read_bytes() for path in project.rglob('*.yaml')}
        if case == 'settings':
            # Exercise preservation of CRLF plus the external LF comment on both
            # hosts. Fixture files themselves remain immutable.
            config = project / 'masterdata.toml'
            config.write_bytes(config.read_bytes().replace(b'\r\n', b'\n').replace(b'\n', b'\r\n'))
        initial_config = (project / 'masterdata.toml').read_bytes()
        executable = binary.resolve()
        if platform.system() == 'Darwin':
            app = temporary / 'MasterData Rewrite.app' / 'Contents'
            (app / 'MacOS').mkdir(parents=True)
            shutil.copy2(executable, app / 'MacOS' / 'masterdata-desktop')
            (app / 'Info.plist').write_bytes(plistlib.dumps({
                # Concurrent installed review bundles must not own this run's
                # activation/focus. Domain identity still comes from Project.
                'CFBundleIdentifier': 'dev.masterdata.evidence.run-' + hashlib.sha256(str(temporary).encode()).hexdigest()[:12],
                'CFBundleName': 'MasterData Rewrite',
                'CFBundleExecutable': 'masterdata-desktop', 'CFBundlePackageType': 'APPL',
                'NSHighResolutionCapable': True,
            }))
            executable = app / 'MacOS' / 'masterdata-desktop'
        report_path = temporary / 'measurement.json'
        if case == 'project':
            removed = temporary / 'remove-entry'
            removed.mkdir()
            (removed / 'notes').write_bytes(b'user bytes')
            (temporary / 'preferences.json').write_text(json.dumps({'recentProjects': [
                {'root': str(project.resolve()), 'name': 'Workspace'},
                {'root': str(removed.resolve()), 'name': 'Remove entry'},
            ]}), encoding='utf-8')
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
            elif phase == 'type-stale':
                file = project / 'sources/ranges.yaml'
                file.write_bytes(file.read_bytes() + b'\n# External type change\n')
            elif phase == 'delivery-target':
                config = project / 'masterdata.toml'
                text = config.read_text(encoding='utf-8').split('\n[build.profiles.development]')[0]
                config.write_text(text + '\n[[publish.targets]]\nkind = "csharp"\npath = "delivery/generated"\n\n[[publish.targets]]\nkind = "binary"\npath = "delivery/masterdata.bytes"\n', encoding='utf-8')
                data.write_bytes(b'kind: [\n')
            elif phase == 'delivery-stale':
                config = project / 'masterdata.toml'
                config.write_bytes(config.read_bytes() + b'\n# External change after preview\n')
                assert not (project / 'delivery').exists(), 'preview created destination'
            elif phase == 'settings-external':
                config = project / 'masterdata.toml'
                config.write_bytes(config.read_bytes() + b'\n# External settings change\n')
            elif phase == 'declaration-stale':
                schema.write_bytes(schema.read_bytes() + b'\n# External table declaration change\n')
            else: raise AssertionError(f'unknown external evidence phase {phase}')
        with (output.parent / 'desktop-process.log').open('w') as log:
            peak = 0
            report = None
            project_args = [] if case == 'project' else ['--project', str(project)]
            process = subprocess.Popen([str(executable), *project_args,
                                        '--evidence-output', str(report_path), '--evidence-kind', case], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 240
                while True:
                    assert process.poll() is None, f'native host exited: {process.returncode}'
                    if case == 'capacity' and platform.system() == 'Windows':
                        peak = max(peak, windows_peak(process))
                    if time.monotonic() >= deadline:
                        raise RuntimeError('native Desktop evidence unavailable: visible focused window did not complete')
                    if report_path.exists():
                        try: report = json.loads(report_path.read_text(encoding='utf-8'))
                        except json.JSONDecodeError: report = None
                        if report is not None:
                            if case in ['external', 'type', 'delivery', 'settings', 'declaration'] and report.get('phase'):
                                phase = report['phase']
                                if phase not in handled:
                                    external_phase(phase)
                                    handled.add(phase)
                                    report_path.with_suffix('.ack').write_text(json.dumps({'phase': phase, 'complete': True}), encoding='utf-8')
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
                if case == 'capacity':
                    oracle = ROOT / 'fixtures/rewrite-oracle/v1/capacity.json'
                    report['implementation']['oracleSha256'] = hashlib.sha256(oracle.read_bytes()).hexdigest()
                    report['dimensions'] = json.loads(oracle.read_text())['input']
                if case in ['migration', 'type', 'declaration']:
                    report['sourceByteEvidence'] = {
                        str(path).replace('\\', '/'): {
                            'before': before.decode('utf-8'),
                            'after': (project / path).read_bytes().decode('utf-8'),
                        } for path, before in initial_sources.items()
                    }
                # A failed assertion must retain the measurements that caused it.
                output.write_text(json.dumps(report, indent=2), encoding='utf-8')
                report['summary'] = {'navigation': check, 'external': check_external, 'creation': check_creation, 'authoring': check_authoring, 'path': check_path, 'migration': check_migration, 'type': check_type, 'delivery': check_delivery, 'capacity': check_capacity, 'tags': check_tags, 'settings': check_settings, 'project': check_project, 'declaration': check_declaration, 'focus': check_focus, 'geometry': check_geometry}[case](report)
                if case == 'project':
                    assert not (temporary / 'cancelled').exists() and not (temporary / 'welcome-cancel').exists(), 'Cancel created a Project'
                    assert (removed / 'notes').read_bytes() == b'user bytes', 'Recent removal modified disk'
                    assert (project / 'masterdata.toml').read_bytes() == initial_config
                    for path, before in initial_sources.items():
                        assert (project / path).read_bytes() == before, path
                    created = temporary / 'created'
                    assert (created / '.gitignore').read_bytes() == b'/.masterdata/\n'
                    assert not (created / '.masterdata').exists() and not (created / '.git').exists() and not (created / 'Assets').exists()
                    files = list((created / 'sources').rglob('*.yaml'))
                    assert len(files) == 2 and sum(b'kind: data' in path.read_bytes() for path in files) == 1
                    assert all(b'records:\n  -' not in path.read_bytes() for path in files), 'Add Row implicitly saved'
                    preferences = json.loads((temporary / 'preferences.json').read_text(encoding='utf-8'))
                    recent = preferences['recentProjects']
                    assert len(recent) == 2 and Path(recent[0]['root']).samefile(created) and Path(recent[1]['root']).samefile(project), 'recent Project order / physical identity mismatch'
                if case == 'declaration':
                    schema = Path('sources/catalog-schema.yaml')
                    nl = b'\r\n' if b'\r\n' in initial_sources[schema] else b'\n'
                    expected = initial_sources[schema].replace(b'key: 0' + nl, b'key: 26' + nl, 1).replace(b'fields: [code, rarity]', b'fields: [rarity, code]')
                    expected += b'references:' + nl + b'  []' + nl + b'\n# External table declaration change\n'
                    assert (project / schema).read_bytes() == expected, 'declaration changed non-target schema bytes'
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items() if path != schema), 'declaration saved record/dependency bytes'
                if case == 'settings':
                    for path, before in initial_sources.items():
                        assert (project / path).read_bytes() == before, path
                    newline = b'\r\n' if b'\r\n' in initial_config else b'\n'
                    addition = b'\n[build.profiles.production]\nexclude_tags = ["debug"]\n\n[[publish.targets]]\nkind = "binary"\npath = "delivery/latest.bytes"\n'
                    expected = initial_config + addition.replace(b'\n', newline) + b'\n# External settings change\n'
                    assert (project / 'masterdata.toml').read_bytes() == expected
                    assert not (project / 'delivery').exists(), 'settings implicitly published'
                    assert not (project / '.masterdata').exists(), 'settings implicitly built'
                if case == 'delivery':
                    artifact = project / '.masterdata/output'
                    receipt = json.loads((artifact / '.masterdata-artifact-set.json').read_text(encoding='utf-8'))
                    assert (project / 'delivery/masterdata.bytes').read_bytes() == (artifact / 'masterdata.bytes').read_bytes()
                    assert hashlib.sha256((artifact / 'masterdata.bytes').read_bytes()).hexdigest() == receipt['binary']['hash']
                    for entry in receipt['csharp']:
                        canonical = artifact / 'csharp' / entry['path']
                        assert hashlib.sha256(canonical.read_bytes()).hexdigest() == entry['hash']
                        assert (project / 'delivery/generated' / entry['path']).read_bytes() == canonical.read_bytes()
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items() if path != Path('sources/catalog-data.yaml')), 'delivery changed source'
                if case == 'external':
                    initial = initial_sources[Path('sources/catalog-data.yaml')]
                    comparison = report['sourceComparison']
                    clean = initial.replace(b'name: Debug Sword', b'name: Outside clean')
                    assert comparison['base'].encode('utf-8') == clean, 'Compare changed exact editing base bytes'
                    assert comparison['before'].encode('utf-8') == initial.replace(b'name: Debug Sword', b'name: Outside dirty'), 'Compare did not capture exact external bytes'
                    assert comparison['after'].encode('utf-8') == clean.replace(b'name: Outside clean', b'name: unfinished input'), 'Compare normalized / changed non-target candidate bytes'
                    expected = initial_sources[Path('sources/catalog-data.yaml')].replace(b'name: Debug Sword', b'name: Outside restored')
                    assert (project / 'sources/catalog-data.yaml').read_bytes() == expected, 'local draft silently overwrote external source'
                elif case == 'path':
                    original = Path('sources/catalog-data.yaml')
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items() if path != original), 'Move saved unrelated source'
                    assert not (project / original).exists()
                    assert (project / report['destination']).read_bytes() == initial_sources[original].replace(b'name: Debug Sword', b'name: Saved before move'), 'Move rewrote source or saved discarded draft'
                    assert not (project / 'sources/cancelled.yaml').exists()
                    assert not (project / '.masterdata').exists(), 'Move implicitly built artifacts'
                elif case == 'migration':
                    schema = Path('sources/catalog-schema.yaml')
                    data = Path('sources/catalog-data.yaml')
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items() if path not in [schema, data]), 'Migration changed unrelated physical sources'
                    schema_nl = b'\r\n' if b'\r\n' in initial_sources[schema] else b'\n'
                    data_nl = b'\r\n' if b'\r\n' in initial_sources[data] else b'\n'
                    old_schema = initial_sources[schema].replace(b'    name: name' + schema_nl, b'    name: title' + schema_nl)
                    expected_schema = old_schema.replace(b'primaryKey:' + schema_nl, schema_nl.join([b'  - key: 18', b'    name: field', b'    type: string', b'    nullable: true', b'primaryKey:', b'']))
                    assert (project / schema).read_bytes() == expected_schema, 'structural authoring rewrote schema presentation or changed MessagePack keys'
                    old_data = initial_sources[data].replace(b'    name:', b'    title:')
                    expected_data = old_data.replace(b'    $tags: [development]' + data_nl, b'    $tags: [development]' + data_nl + b'    field: null' + data_nl).replace(b'    $tags: [production]' + data_nl, b'    $tags: [production]' + data_nl + b'    field: null' + data_nl)
                    assert (project / data).read_bytes() == expected_data, 'structural authoring changed non-target record bytes'
                    assert not (project / '.masterdata/output').exists(), 'Migration implicitly built artifacts'
                elif case == 'type':
                    source = Path('sources/item-id.yaml')
                    before = initial_sources[source]
                    nl = b'\r\n' if b'\r\n' in before else b'\n'
                    expected = before + nl.join([b'  conversions:', b'    fromUnderlyingImplicit: true', b'    toUnderlyingImplicit: false', b''])
                    assert (project / source).read_bytes() == expected, 'conversion change altered source representation'
                    ranges = Path('sources/ranges.yaml')
                    assert (project / ranges).read_bytes() == initial_sources[ranges] + b'\n# External type change\n', 'stale Plan overwrote the external change'
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items() if path not in [source, ranges]), 'inverse Type operations failed exact restoration or wrote a draft'
                    assert not (project / '.masterdata/output').exists(), 'Type Apply implicitly built artifacts'
                elif case == 'delivery':
                    assert (project / 'sources/catalog-data.yaml').read_bytes() == b'kind: [\n', 'Publish touched externally invalid YAML'
                elif case == 'declaration':
                    assert not (project / '.masterdata/output').exists(), 'declaration Apply implicitly built artifacts'
                else:
                    assert all((project / path).read_bytes() == value for path, value in initial_sources.items()), 'interaction implicitly wrote source'
                if case == 'capacity':
                    assert not (project / '.masterdata/output').exists(), 'capacity interaction implicitly built artifacts'
                    report['diskUnchanged'] = True
                if case == 'creation':
                    new_sources = {str(path.relative_to(project)).replace('\\', '/') for pattern in ('*.yaml', '*.yml') for path in project.rglob(pattern)} - {str(path).replace('\\', '/') for path in initial_sources}
                    assert new_sources == set(report['created']), 'implicit / missing created source'
                    assert not (project / 'sources/cancelled.yaml').exists()
                    assert 'table: fresh-table' in (project / 'sources/fresh-table.yaml').read_text(encoding='utf-8')
                    assert 'table: fresh-table' in (project / 'sources/catalog-new/storage-name.yml').read_text(encoding='utf-8')
                    assert 'value: 18446744073709551615' in (project / 'sources/catalog-new/huge-token.yaml').read_text(encoding='utf-8')
                    assert not (project / 'artifacts').exists(), 'creation implicitly built artifacts'
                output.write_text(json.dumps(report, indent=2), encoding='utf-8')
                print(json.dumps(report['summary'], indent=2))
            finally:
                if case == 'capacity' and platform.system() == 'Windows':
                    peak = max(peak, windows_peak(process))
                process.terminate()
                process.wait(timeout=10)
                if case == 'capacity' and report is not None:
                    peak, boundary = child_peak(peak)
                    report['peakRssBytes'] = peak
                    report['peakRssBoundary'] = boundary
                    output.write_text(json.dumps(report, indent=2), encoding='utf-8')
                    assert peak > 0, 'peak RSS unavailable'


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--case', choices=['navigation', 'authoring', 'external', 'creation', 'path', 'migration', 'type', 'delivery', 'capacity', 'tags', 'settings', 'project', 'declaration', 'focus', 'geometry'], default='navigation')
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(ROOT / 'fixtures'):
        parser.error('frozen fixture cannot be output')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    run(args.binary, args.output, args.case)
