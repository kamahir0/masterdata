// Visual copies never retain editor identity, accessible controls or event handlers.
export function visualCopy(
  item: HTMLElement,
  clip?: { left: number; right: number; top: number; bottom: number },
) {
  const origin = item.getBoundingClientRect();
  const row = item.cloneNode(true) as HTMLElement;
  const originals = [item, ...item.querySelectorAll<HTMLElement>("*")];
  const copies = [row, ...row.querySelectorAll<HTMLElement>("*")];
  // Wide virtual rows still mount offscreen columns. Keep their geometry out
  // of the snapshot work; only visible direct branches need computed styles.
  const branches = [...row.children].map((copy, index) => ({
    copy: copy as HTMLElement,
    box: item.children[index].getBoundingClientRect(),
  }));
  if (clip)
    branches.forEach(({ copy, box }) => {
      if (
        box.right <= clip.left ||
        box.left >= clip.right ||
        box.bottom <= clip.top ||
        box.top >= clip.bottom
      )
        copy.remove();
    });
  const properties = [
    "display",
    "box-sizing",
    "font",
    "color",
    "background",
    "border",
    "border-radius",
    "padding",
    "margin",
    "gap",
    "align-items",
    "justify-content",
    "flex-direction",
    "flex-grow",
    "flex-shrink",
    "flex-basis",
    "overflow",
    "white-space",
    "text-align",
    "line-height",
    "box-shadow",
    "opacity",
  ];
  originals.forEach((element, index) => {
    const copy = copies[index];
    if (copy !== row && !row.contains(copy)) return;
    const style = getComputedStyle(element),
      box = element.getBoundingClientRect();
    properties.forEach((property) =>
      copy.style.setProperty(property, style.getPropertyValue(property)),
    );
    copy.style.width = `${box.width}px`;
    copy.style.height = `${box.height}px`;
    copy.removeAttribute("aria-label");
    for (const attribute of [...copy.attributes])
      if (
        attribute.name.startsWith("data-sort-") ||
        attribute.name === "data-row-key" ||
        attribute.name === "data-motion-key" ||
        attribute.name === "id"
      )
        copy.removeAttribute(attribute.name);
    if (element instanceof HTMLInputElement && copy instanceof HTMLInputElement)
      copy.value = element.value;
  });
  branches.forEach(({ copy, box }) => {
    Object.assign(copy.style, {
      position: "absolute",
      left: `${box.left - origin.left}px`,
      top: `${box.top - origin.top}px`,
    });
  });
  return row;
}
