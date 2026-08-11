// A hover tooltip that portals the label to the end of <body> with `position: fixed`, so it is
// never clipped by a scroll container or stacked under later siblings the way an absolutely
// positioned child is. Reads the current label from the node's `data-tip` attribute at mouseenter,
// so the text can be reactive without restarting the action.
//
// Usage: <span use:tooltip data-tip="...">hover me</span>
export function tooltip(node: HTMLElement) {
  let tip: HTMLDivElement | null = null;

  function label(): string {
    return node.getAttribute("data-tip") ?? "";
  }

  function hide() {
    tip?.remove();
    tip = null;
  }

  function show() {
    hide();
    tip = document.createElement("div");
    tip.textContent = label();
    tip.style.cssText = `
      position: fixed;
      z-index: 99999;
      pointer-events: none;
      white-space: nowrap;
      background: #171717;
      color: #d4d4d4;
      border: 1px solid #404040;
      border-radius: 6px;
      padding: 4px 8px;
      font-size: 12px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    `;
    document.body.appendChild(tip);
    position();
  }

  // Right-align the tooltip to the node's right edge, floated just above it. If there is not room
  // above (a row near the top of the viewport) it drops below instead of going off-screen.
  function position() {
    if (!tip) return;
    const rect = node.getBoundingClientRect();
    const top = rect.top - tip.offsetHeight - 6;
    tip.style.left = `${Math.max(0, rect.right - tip.offsetWidth)}px`;
    tip.style.top = `${top >= 0 ? top : rect.bottom + 6}px`;
  }

  function onEnter() {
    show();
  }
  function onLeave() {
    hide();
  }

  node.addEventListener("mouseenter", onEnter);
  node.addEventListener("mouseleave", onLeave);
  // Re-seat the tooltip alongside the node as either scrolls, so it does not drift off the chip.
  node.addEventListener("scroll", position, true);
  window.addEventListener("scroll", position, true);

  return {
    destroy() {
      node.removeEventListener("mouseenter", onEnter);
      node.removeEventListener("mouseleave", onLeave);
      node.removeEventListener("scroll", position, true);
      window.removeEventListener("scroll", position, true);
      hide();
    },
  };
}
