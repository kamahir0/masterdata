import { afterEach, expect, test, vi } from 'vitest';
import { startGridReorder } from '../src/grid-reorder';

const rect = (left: number, top: number, width: number, height: number) =>
  ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top } as DOMRect);
let cleanup: (() => void) | undefined;
afterEach(() => { cleanup?.(); cleanup = undefined; document.body.replaceChildren(); vi.restoreAllMocks(); vi.useRealTimers(); });
function grid(widths = [100, 100, 100], rowCount = 3) {
  const scroll = document.createElement('div'); scroll.className = 'grid-scroll';
  const root = document.createElement('table'); root.className = 'record-grid';
  root.innerHTML = `<thead><tr><th class="row-number">#</th>${widths.map((_, i) => `<th data-column-index="${i}"><input id="field-${i}" aria-label="Field ${i}" value="field${i}" /></th>`).join('')}</tr></thead><tbody></tbody>`;
  scroll.append(root); document.body.append(scroll);
  Object.defineProperties(scroll, { clientWidth: { value: 400 }, clientHeight: { value: 250 } });
  vi.spyOn(scroll, 'getBoundingClientRect').mockReturnValue(rect(0, 0, 400, 250));
  vi.spyOn(root.tHead!, 'getBoundingClientRect').mockReturnValue(rect(0, 0, 400, 62));
  vi.spyOn(root.querySelector<HTMLElement>('.row-number')!, 'getBoundingClientRect').mockReturnValue(rect(0, 0, 60, 62));
  let x = 60;
  const columns = [...root.querySelectorAll<HTMLElement>('[data-column-index]')];
  columns.forEach((col, i) => { const left = x; vi.spyOn(col, 'getBoundingClientRect').mockImplementation(() => rect(left - scroll.scrollLeft, 0, widths[i], 62)); x += widths[i]; });
  const addRow = (i: number) => {
    const row = root.tBodies[0].insertRow(); row.dataset.gridRowIndex = String(i);
    row.innerHTML = `<th class="row-number">${i}</th>${widths.map((_, j) => `<td data-column-index="${j}"><div data-cell="${i}:${j}" role="gridcell" tabindex="0">row${i} col${j}</div></td>`).join('')}`;
    vi.spyOn(row, 'getBoundingClientRect').mockImplementation(() => rect(0, 62 + i * 32 - scroll.scrollTop, 360, 32));
    let left = 0;
    [...row.children].forEach((cell, j) => { const width = j ? widths[j - 1] : 60, cellLeft = left;
      vi.spyOn(cell, 'getBoundingClientRect').mockImplementation(() => rect(j ? cellLeft - scroll.scrollLeft : 0, 62 + i * 32 - scroll.scrollTop, width, 32)); left += width; });
    return row;
  };
  const rows = Array.from({ length: rowCount }, (_, i) => addRow(i));
  return { root, scroll, columns, rows, addRow };
}
function pointer(type: string, x: number, y: number, id = 7) {
  const event = new MouseEvent(type, { clientX: x, clientY: y, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'pointerId', { value: id }); window.dispatchEvent(event);
}
function begin(g: ReturnType<typeof grid>, axis: 'x' | 'y', index: number, x: number, y: number, itemCount = 3) {
  const onDrop = vi.fn();
  cleanup = startGridReorder({ pointerId: 7, clientX: x, clientY: y }, { root: g.root, axis, sourceIndex: index, itemCount, rowHeight: 32, onDrop });
  return onDrop;
}

test('column preview moves the header and body together without changing DOM order or issuing an intent', () => {
  const g = grid(); const drop = begin(g, 'x', 0, 110, 20);
  pointer('pointermove', 330, 20);
  expect(g.root.dataset.dragDestination).toBe('2');
  expect(g.columns[1].style.transform).toBe('translateX(-100px)');
  expect(g.rows[0].children[2].getAttribute('style')).toContain('translateX(-100px)');
  expect(g.root.querySelectorAll('.grid-drag-source')).toHaveLength(4);
  expect(document.querySelector('.grid-drag-ghost')?.getAttribute('style')).toContain('translateX(220px)');
  expect([...g.root.querySelectorAll('thead th[data-column-index]')]).toEqual(g.columns);
  expect(drop).not.toHaveBeenCalled();
  const overlay = document.querySelector('.grid-drag-overlay')!;
  expect(overlay.getAttribute('aria-hidden')).toBe('true');
  expect(overlay.querySelector('[id], [data-cell], [data-column-index]')).toBeNull();
  pointer('pointerup', 330, 20);
  expect(drop).toHaveBeenCalledExactlyOnceWith(2);
  expect(document.querySelector('.grid-drag-overlay')).toBeNull();
  expect(g.columns[1].style.transform).toBe('');
});

test('unequal width columns keep a stable preview when animated targets move under a stationary pointer', () => {
  const g = grid([160, 70, 100]); const drop = begin(g, 'x', 0, 140, 20);
  pointer('pointermove', 320, 20);
  const destination = g.root.dataset.dragDestination;
  vi.mocked(g.columns[1].getBoundingClientRect).mockReturnValue(rect(60, 0, 70, 62));
  pointer('pointermove', 320, 20);
  expect(g.root.dataset.dragDestination).toBe(destination);
  pointer('pointermove', 140, 20);
  expect(g.root.dataset.dragDestination).toBe('0');
  pointer('pointerup', 140, 20);
  expect(drop).not.toHaveBeenCalled();
});

test.each(['pointercancel', 'escape', 'blur', 'outside', 'dispose'])('%s cancels and restores all preview state', (mode) => {
  const g = grid(); const drop = begin(g, 'y', 2, 30, 142);
  pointer('pointermove', 30, 80);
  expect(g.rows[0].style.transform).toBe('translateY(32px)');
  expect(g.rows[2].classList.contains('grid-drag-source')).toBe(true);
  if (mode === 'escape') window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  else if (mode === 'blur') window.dispatchEvent(new Event('blur'));
  else if (mode === 'outside') pointer('pointerup', 500, 80);
  else if (mode === 'dispose') cleanup!();
  else pointer('pointercancel', 30, 80);
  pointer('pointerup', 30, 80);
  expect(drop).not.toHaveBeenCalled();
  expect(g.root.dataset.dragDestination).toBeUndefined();
  expect(g.root.querySelector('.grid-reorder-item, .grid-drag-source')).toBeNull();
  expect(g.rows[0].style.transform).toBe('');
  expect(document.querySelector('.grid-drag-overlay')).toBeNull();
});

test('click threshold and another pointer cannot activate or commit a reorder', () => {
  const g = grid(); const drop = begin(g, 'y', 0, 30, 78);
  pointer('pointermove', 30, 200, 8);
  pointer('pointermove', 30, 81);
  expect(document.querySelector('.grid-drag-overlay')).toBeNull();
  pointer('pointerup', 30, 81);
  expect(drop).not.toHaveBeenCalled();
});

test('row preview survives virtual removal of the source and styles newly rendered rows using source positions', async () => {
  const g = grid(); const drop = begin(g, 'y', 0, 30, 78, 1000);
  pointer('pointermove', 30, 180);
  g.scroll.scrollTop = 640; g.rows.forEach(row => row.remove());
  const newlyVisible = [20, 21, 22].map(g.addRow);
  g.scroll.dispatchEvent(new Event('scroll'));
  await Promise.resolve();
  expect(g.root.dataset.dragDestination).toBe('23');
  expect(newlyVisible[0].style.transform).toBe('translateY(-32px)');
  expect(document.querySelector('.grid-drag-overlay')?.textContent).toContain('row0 col0');
  expect(g.root.querySelectorAll('[data-grid-row-index]')).toHaveLength(3);
  expect(drop).not.toHaveBeenCalled();
  pointer('pointerup', 30, 180);
  expect(drop).toHaveBeenCalledExactlyOnceWith(23);
});

test('edge scrolling changes logical destination while the ghost stays under the pointer', () => {
  vi.useFakeTimers(); const g = grid(); const drop = begin(g, 'y', 0, 30, 78, 1000);
  pointer('pointermove', 30, 240);
  const before = Number(g.root.dataset.dragDestination);
  vi.advanceTimersByTime(90);
  expect(g.scroll.scrollTop).toBe(54);
  expect(Number(g.root.dataset.dragDestination)).toBeGreaterThan(before);
  expect(document.querySelector('.grid-drag-ghost')?.getAttribute('style')).toContain('translateY(162px)');
  expect(drop).not.toHaveBeenCalled();
});

test('horizontal scrolling adjusts a column destination and refreshes visible column cells', () => {
  const g = grid(); begin(g, 'x', 0, 110, 20);
  pointer('pointermove', 190, 20);
  g.scroll.scrollLeft = 180; g.scroll.dispatchEvent(new Event('scroll'));
  expect(g.root.dataset.dragDestination).toBe('2');
  expect(document.querySelectorAll('.grid-drag-copy')).toHaveLength(4);
  expect(document.querySelector('.grid-drag-overlay')?.getAttribute('style')).toContain('left: 60px');
});
