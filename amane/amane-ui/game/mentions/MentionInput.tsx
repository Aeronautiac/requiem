// The message composer with @-mention autocomplete AND live chips. A contenteditable rather than
// an <input>, because a mention has to sit in the text as one atomic, non-editable pill: typing `@`
// opens a narrowing list of players/roles/orgs/System, and choosing one drops a chip in place of
// the `@query`. The editable content is the source of truth; `value` is the token string it
// serialises to (`@<player:3:0>` …), which is what actually gets sent and what MentionText later
// re-parses back into chips.
//
// Enter submits when the caller wants that (a chat composer); Shift+Enter always inserts a literal
// newline, and plain Enter does too when there is nothing to submit to (a free-text form field) or
// on a touch screen.
import { ROLES } from "amane-client/bindings.ts";
import { mentionToken, refLabel } from "amane-client/text.ts";
import type { Mention } from "amane-client/text.ts";
import { useEffect, useRef, useState } from "react";
import { refColor, tint } from "../../style.ts";
import { CHIP_CLASS, Entity } from "../Name.tsx";
import { useView } from "../game_ui.ts";

type Candidate = { mention: Mention; label: string };

// Letters and digits only, lowercased. Both the label and the query go through this, so spaces and
// punctuation never matter: "kiras", "kira's" and "kiras kingdom" all find "Kira's Kingdom".
function normalize(s: string): string {
  return s.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "");
}

// How well a label answers a normalized query: 0 exact, 1 prefix, 2 the query starts at a later
// word ("king" finds "Kira's Kingdom"), 3 anywhere, -1 no match. Ranking across all kinds is what
// stops a long name that merely CONTAINS "l" from burying the role "L".
function rank(label: string, q: string): number {
  const whole = normalize(label);
  if (whole === q) return 0;
  if (whole.startsWith(q)) return 1;
  const words = label.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
  for (let i = 1; i < words.length; i++) {
    if (words.slice(i).join("").startsWith(q)) return 2;
  }
  return whole.includes(q) ? 3 : -1;
}

// The mention being typed at the caret: the nearest `@` that starts a word, and everything typed
// since. It may run across spaces, since names do; whether it is still a mention is decided by
// whether anything matches it, and the list closes by itself once nothing does. A newline, or
// `<>@` (the inside of a token), ends it. Operates within the caret's own text node: a chip is a
// hard boundary.
function activeQuery(text: string, cursor: number): { start: number; query: string } | null {
  for (let i = cursor - 1; i >= 0; i--) {
    const c = text[i];
    if (c === "@") {
      if (i > 0 && !/\s/.test(text[i - 1])) return null;
      const q = text.slice(i + 1, cursor);
      return /[\n<>@]/.test(q) ? null : { start: i, query: q };
    }
    if (c === "\n") return null;
  }
  return null;
}

export function MentionInput({
  value,
  onChange,
  onSubmit,
  placeholder,
  disabled = false,
}: {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: () => void;
  placeholder?: string;
  disabled?: boolean;
}) {
  const view = useView();

  const elRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  // The text run being replaced when a candidate is chosen, captured at the last refresh.
  const queryNode = useRef<Text | null>(null);
  const queryStart = useRef(0);
  const queryEnd = useRef(0);

  const chipColor = (m: Mention) => refColor(m, view);

  // Walk the editable children into the token string: text nodes verbatim, chips as their token.
  function serialize(): string {
    let out = "";
    elRef.current?.childNodes.forEach((node) => {
      if (node.nodeType === Node.TEXT_NODE) out += node.textContent ?? "";
      else if (node instanceof HTMLElement) out += node.dataset.token ?? node.textContent ?? "";
    });
    return out;
  }

  function caretText(): { node: Text; offset: number } | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return null;
    const node = sel.anchorNode;
    if (!node || !elRef.current?.contains(node) || node.nodeType !== Node.TEXT_NODE) return null;
    return { node: node as Text, offset: sel.anchorOffset };
  }

  function refresh() {
    const cc = caretText();
    const found = cc && activeQuery(cc.node.textContent ?? "", cc.offset);
    if (!cc || !found) {
      setOpen(false);
      return;
    }
    if (found.query !== query || !open) setIndex(0);
    setOpen(true);
    setQuery(found.query);
    queryNode.current = cc.node;
    queryStart.current = found.start;
    queryEnd.current = cc.offset;
  }

  function oninput() {
    onChange(serialize());
    refresh();
  }

  const candidates: Candidate[] = (() => {
    if (!open) return [];
    const q = normalize(query);
    // Kept in this order so a full tie falls back to players, then roles, then orgs, then System.
    const scored: { candidate: Candidate; rank: number }[] = [];
    const add = (candidate: Candidate) => {
      const r = rank(candidate.label, q);
      if (r >= 0) scored.push({ candidate, rank: r });
    };

    for (const id of view.players.keys()) {
      const mention: Mention = { kind: "player", id };
      add({ mention, label: refLabel(mention, view.players) });
    }
    for (const role of ROLES) {
      const mention: Mention = { kind: "role", role };
      add({ mention, label: refLabel(mention, view.players) });
    }
    const seen = new Set<string>();
    for (const org of view.orgs.values()) {
      if (seen.has(org.name)) continue;
      seen.add(org.name);
      const mention: Mention = { kind: "org", org: org.name };
      add({ mention, label: refLabel(mention, view.players) });
    }
    add({ mention: { kind: "news_anchor" }, label: refLabel({ kind: "news_anchor" }, view.players) });
    add({ mention: { kind: "press_conference" }, label: refLabel({ kind: "press_conference" }, view.players) });
    add({ mention: { kind: "system" }, label: refLabel({ kind: "system" }, view.players) });

    // Better match first; on a tie the shorter label (so "L" beats "Lawliet"); then source order.
    scored.sort((a, b) => a.rank - b.rank || a.candidate.label.length - b.candidate.label.length);
    return scored.slice(0, 8).map((s) => s.candidate);
  })();

  function choose(c: Candidate) {
    const node = queryNode.current;
    if (!node) return;
    const text = node.textContent ?? "";
    const chip = document.createElement("span");
    chip.dataset.token = mentionToken(c.mention);
    chip.contentEditable = "false";
    chip.className = CHIP_CLASS;
    const style = tint(chipColor(c.mention));
    chip.style.color = style.color as string;
    chip.style.backgroundColor = style.backgroundColor as string;
    chip.textContent = c.label;

    const space = document.createTextNode(" ");
    const after = document.createTextNode(text.slice(queryEnd.current));
    node.textContent = text.slice(0, queryStart.current);
    node.after(chip, space, after);

    setOpen(false);
    const sel = window.getSelection();
    const range = document.createRange();
    range.setStart(after, 0); // caret just past the inserted space
    range.collapse(true);
    sel?.removeAllRanges();
    sel?.addRange(range);
    elRef.current?.focus();
    onChange(serialize());
  }

  // Inserts plain text at the caret (replacing any selection), whether the caret sits inside a text
  // node or directly in the (possibly empty) editable element. Every newline and every paste comes
  // through here, so the editable only ever holds text nodes, chips, and the one trailing <br>.
  function insertText(text: string) {
    const el = elRef.current;
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !el) return;
    const range = sel.getRangeAt(0);
    if (!el.contains(range.commonAncestorContainer)) return;
    range.deleteContents();
    const node = document.createTextNode(text);
    range.insertNode(node);
    range.setStartAfter(node);
    range.collapse(true);
    sel.removeAllRanges();
    sel.addRange(range);
    // A newline that ends the content draws no line of its own, so the caret would stay put and
    // Shift+Enter would look like it did nothing. A <br> kept last gives that line a box; after
    // anything else it draws nothing. It has no token, so it serializes to nothing.
    if (el.lastChild?.nodeName !== "BR") el.append(document.createElement("br"));
    onChange(serialize());
    refresh();
  }

  // The browser's own paste brings HTML (divs, spans, <br>s) that serialize() can't read back into
  // text. Paste the plain text instead.
  function onpaste(e: React.ClipboardEvent) {
    e.preventDefault();
    insertText(e.clipboardData.getData("text/plain").replace(/\r\n?/g, "\n"));
  }

  function onkeydown(e: React.KeyboardEvent) {
    if (open && candidates.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setIndex((index + 1) % candidates.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setIndex((index - 1 + candidates.length) % candidates.length);
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        choose(candidates[index]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false);
        return;
      }
    }
    if (e.key === "Enter") {
      e.preventDefault();
      // A phone keyboard has no Shift+Enter, so there Enter is the newline and the caller's own
      // button sends.
      if (e.shiftKey || !onSubmit || matchMedia("(pointer: coarse)").matches) {
        insertText("\n");
      } else {
        onSubmit();
      }
    }
  }

  // The parent clears `value` after a send; mirror that back into the editable, which the input
  // handler alone would never do (nothing typed to trigger it). This is the only value→DOM path;
  // otherwise the DOM leads.
  useEffect(() => {
    const el = elRef.current;
    if (value === "" && el && (el.textContent !== "" || el.querySelector("[data-token]"))) {
      el.replaceChildren();
    }
  }, [value]);

  // The top edge of the visible text box to anchor the dropdown to: the nearest ancestor that
  // actually draws a background, walking up from the editable itself.
  function boxTop(node: HTMLElement): number {
    let cur: HTMLElement | null = node;
    while (cur) {
      const bg = getComputedStyle(cur).backgroundColor;
      if (bg && bg !== "transparent" && bg !== "rgba(0, 0, 0, 0)") return cur.getBoundingClientRect().top;
      cur = cur.parentElement;
    }
    return node.getBoundingClientRect().top;
  }

  // Position the popover flush at the top edge of the visible text box, growing upward. A manual
  // popover lives in the browser's top layer, so no ancestor (a panel, a channel scroller, even a
  // `showModal()` dialog) can clip it or paint over it. Pinning with `bottom` needs no height
  // measurement, so there is nothing to mis-measure mid-layout.
  function placePopover() {
    const node = listRef.current;
    const anchor = elRef.current;
    if (!node || !anchor) return;
    const r = anchor.getBoundingClientRect();
    const width = Math.max(r.width, 288);
    node.style.position = "fixed";
    node.style.margin = "0";
    node.style.width = `${Math.min(width, window.innerWidth - 12)}px`;
    let left = r.left;
    if (left + width > window.innerWidth - 12) left = window.innerWidth - 12 - width;
    node.style.left = `${Math.max(6, left)}px`;
    node.style.right = "auto";
    node.style.top = "auto";
    node.style.bottom = `${window.innerHeight - boxTop(anchor)}px`;
  }

  useEffect(() => {
    const node = listRef.current;
    if (!open || candidates.length === 0 || !node) return;
    if (!node.matches(":popover-open")) node.showPopover();
    placePopover();
    window.addEventListener("scroll", placePopover, true);
    window.addEventListener("resize", placePopover);
    return () => {
      window.removeEventListener("scroll", placePopover, true);
      window.removeEventListener("resize", placePopover);
      if (node.matches(":popover-open")) node.hidePopover();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, candidates.length]);

  return (
    <div className="relative w-full">
      {open && candidates.length > 0 && (
        <ul
          ref={listRef}
          popover="manual"
          className="max-h-56 overflow-y-auto border border-edge bg-panel px-0 py-1 shadow-lg"
        >
          {candidates.map((c, i) => (
            <li key={c.mention.kind + mentionToken(c.mention)}>
              <button
                type="button"
                className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm ${i === index ? "bg-raised" : "hover:bg-raised"}`}
                onMouseDown={(e) => {
                  e.preventDefault();
                  choose(c);
                }}
              >
                <span className="truncate">
                  <Entity of={c.mention} view={view} menu={false} />
                </span>
                <span className="ml-auto px-1 text-xs uppercase tracking-wide" style={tint(chipColor(c.mention))}>
                  {c.mention.kind}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {/* The placeholder is drawn inside the editable, as a zero-height float ahead of the caret, so
          it shares the caret's line box and can't sit above or beside it. */}
      <div
        ref={elRef}
        contentEditable={!disabled}
        role="textbox"
        tabIndex={disabled ? -1 : 0}
        aria-multiline="true"
        aria-disabled={disabled}
        aria-label={placeholder}
        onInput={oninput}
        onKeyDown={onkeydown}
        onPaste={onpaste}
        onKeyUp={refresh}
        onClick={refresh}
        onBlur={() => setOpen(false)}
        data-placeholder={placeholder}
        className={`min-h-9 max-h-48 w-full overflow-y-auto whitespace-pre-wrap break-words border border-edge bg-panel px-2.5 py-2 text-sm pointer-coarse:text-base text-ink focus:outline-none ${value === "" ? "before:pointer-events-none before:float-left before:h-0 before:text-ink-dim before:content-[attr(data-placeholder)]" : ""} ${disabled ? "opacity-50" : ""}`}
      />
    </div>
  );
}
