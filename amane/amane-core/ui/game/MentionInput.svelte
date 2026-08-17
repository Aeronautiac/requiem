<script lang="ts">
  // The message composer with @-mention autocomplete AND live chips. A contenteditable rather than
  // an <input>, because a mention has to sit in the text as one atomic, non-editable pill: typing
  // `@` opens a narrowing list of players/roles/orgs/System, and choosing one drops a chip in place
  // of the `@query`. The editable content is the source of truth; `value` is the token string it
  // serialises to (`@<player:3:0>` …), which is what actually gets sent and what MentionText later
  // re-parses back into chips.
  import { ROLES } from "../../constants";
  import type { Mention } from "../../game/helpers.svelte";
  import type { Statuses } from "../../bindings";
  import {
    chipStyle,
    mentionChipColorVar,
    mentionLabel,
    mentionToken,
    orgDisplayName,
    playerLabel,
    roleLabel,
    t,
  } from "../../game/helpers.svelte";
  import type { Org, Player } from "../../game/types";

  interface Props {
    value: string;
    players: ReadonlyMap<string, Player>;
    orgs: ReadonlyMap<string, Org>;
    // Public status so a player candidate's chip is coloured like their name is everywhere else.
    newsAnchor: string | null;
    pressConf: ReadonlySet<string>;
    statuses: ReadonlyMap<string, Statuses>;
    placeholder?: string;
    // A standalone prose box (announcement, death message) rather than the cramped composer field:
    // visibly bordered, taller, and full width so it reads as a real text box.
    boxed?: boolean;
    onsubmit: () => void;
  }
  let {
    value = $bindable(),
    players,
    orgs,
    newsAnchor,
    pressConf,
    statuses,
    placeholder,
    boxed = false,
    onsubmit,
  }: Props = $props();

  type Candidate = { mention: Mention; label: string };

  const chipColor = (m: Mention) =>
    mentionChipColorVar(m, { news_anchor: newsAnchor, press_conf: pressConf, actor_statuses: statuses });

  let el = $state<HTMLDivElement>();
  let open = $state(false);
  let query = $state("");
  let index = $state(0);
  // The text run being replaced when a candidate is chosen, captured at the last refresh.
  let queryNode: Text | null = null;
  let queryStart = 0;
  let queryEnd = 0;

  // Walk the editable children into the token string: text nodes verbatim, chips as their token.
  function serialize(): string {
    let out = "";
    el?.childNodes.forEach((node) => {
      if (node.nodeType === Node.TEXT_NODE) out += node.textContent ?? "";
      else if (node instanceof HTMLElement) out += node.dataset.token ?? node.textContent ?? "";
    });
    return out;
  }

  // The mention being typed at the caret: the last `@` starting a word, with only a plain word
  // after it. `<>@` or whitespace means it is not a fresh query (e.g. sits inside a token), so the
  // list stays shut. Operates within the caret's own text node — a chip is a hard boundary.
  function activeQuery(text: string, cursor: number): { start: number; query: string } | null {
    for (let i = cursor - 1; i >= 0; i--) {
      const c = text[i];
      if (c === "@") {
        if (i > 0 && !/\s/.test(text[i - 1])) return null;
        const q = text.slice(i + 1, cursor);
        return /[\s<>@]/.test(q) ? null : { start: i, query: q };
      }
      if (/\s/.test(c)) return null;
    }
    return null;
  }

  function caretText(): { node: Text; offset: number } | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return null;
    const node = sel.anchorNode;
    if (!node || !el?.contains(node) || node.nodeType !== Node.TEXT_NODE) return null;
    return { node: node as Text, offset: sel.anchorOffset };
  }

  function refresh() {
    const cc = caretText();
    const found = cc && activeQuery(cc.node.textContent ?? "", cc.offset);
    if (!cc || !found) {
      open = false;
      return;
    }
    if (found.query !== query || !open) index = 0;
    open = true;
    query = found.query;
    queryNode = cc.node;
    queryStart = found.start;
    queryEnd = cc.offset;
  }

  function oninput() {
    value = serialize();
    refresh();
  }

  // How well a label answers the query: 0 exact, 1 prefix, 2 the query starts a word inside it,
  // 3 anywhere, -1 no match. Ranking across all kinds is what stops a long name that merely CONTAINS
  // "l" from burying the role "L" — the exact hit wins wherever it sits in the source order.
  function rank(label: string, q: string): number {
    const l = label.toLowerCase();
    if (l === q) return 0;
    if (l.startsWith(q)) return 1;
    for (let i = l.indexOf(q); i > 0; i = l.indexOf(q, i + 1)) {
      if (!/[a-z0-9]/i.test(l[i - 1])) return 2;
    }
    return l.includes(q) ? 3 : -1;
  }

  const candidates = $derived.by((): Candidate[] => {
    if (!open) return [];
    const q = query.toLowerCase();
    // Kept in this order so a full tie falls back to players, then roles, then orgs, then System.
    const scored: { candidate: Candidate; rank: number }[] = [];
    const add = (candidate: Candidate) => {
      const r = rank(candidate.label, q);
      if (r >= 0) scored.push({ candidate, rank: r });
    };

    for (const id of players.keys()) {
      add({ mention: { kind: "player", id }, label: playerLabel(id, players) });
    }
    for (const role of ROLES) {
      add({ mention: { kind: "role", role }, label: roleLabel(role) });
    }
    const seen = new Set<string>();
    for (const org of orgs.values()) {
      if (seen.has(org.name)) continue;
      seen.add(org.name);
      add({ mention: { kind: "org", org: org.name }, label: orgDisplayName(org.name) });
    }
    add({ mention: { kind: "news_anchor" }, label: t("news_anchor_label") });
    add({ mention: { kind: "press_conference" }, label: t("press_conference_label") });
    add({ mention: { kind: "system" }, label: t("display_system") });

    // Better match first; on a tie the shorter label (so "L" beats "Lawliet"); then source order.
    scored.sort(
      (a, b) => a.rank - b.rank || a.candidate.label.length - b.candidate.label.length,
    );
    return scored.slice(0, 8).map((s) => s.candidate);
  });

  function choose(c: Candidate) {
    if (!queryNode) return;
    const text = queryNode.textContent ?? "";
    const chip = document.createElement("span");
    chip.dataset.token = mentionToken(c.mention);
    chip.contentEditable = "false";
    chip.className = "box-decoration-clone rounded px-0.5 py-0.5 font-medium";
    chip.style.cssText = chipStyle(chipColor(c.mention));
    chip.textContent = c.label;

    const space = document.createTextNode(" ");
    const after = document.createTextNode(text.slice(queryEnd));
    queryNode.textContent = text.slice(0, queryStart);
    queryNode.after(chip, space, after);

    open = false;
    const sel = window.getSelection();
    const range = document.createRange();
    range.setStart(after, 0); // caret just past the inserted space
    range.collapse(true);
    sel?.removeAllRanges();
    sel?.addRange(range);
    el?.focus();
    value = serialize();
  }

  function onkeydown(e: KeyboardEvent) {
    if (open && candidates.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        index = (index + 1) % candidates.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        index = (index - 1 + candidates.length) % candidates.length;
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        choose(candidates[index]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        open = false;
        return;
      }
    }
    // A single-line composer: Enter sends, it never inserts a newline.
    if (e.key === "Enter") {
      e.preventDefault();
      onsubmit();
    }
  }

  // The parent clears `value` after a send; mirror that back into the editable, which the input
  // handler alone would never do (nothing typed to trigger it). This is the only value→DOM path;
  // otherwise the DOM leads.
  $effect(() => {
    if (value === "" && el && (el.textContent !== "" || el.querySelector("[data-token]"))) {
      el.replaceChildren();
    }
  });

  // The candidate list node, once its #if mounts it.
  let listEl = $state<HTMLUListElement>();

  // The top edge of the visible text box to anchor the dropdown to. In a boxed field the editable
  // IS the box; in the chat composer the editable sits a few px inside the bordered bar (it is
  // vertically centred), so pinning to the editable's top would overlap the box's top edge -- ugly.
  // Walk up to the nearest ancestor that actually draws a background, which is the box a reader can
  // see, and anchor there.
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
    const node = listEl;
    const anchor = el;
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

  $effect(() => {
    const node = listEl;
    // The #if removes the list the moment there are no candidates, so existence is the only thing
    // to guard on here -- deliberately not `candidates.length`, which would re-run this effect (and
    // flicker the popover) on every keystroke. Since the list's bottom is pinned to the input, its
    // content growing or shrinking never needs re-placing.
    if (!open || !node) return;
    // Unless it is already open (a re-run as the candidate list changes), put it in the top layer.
    if (!node.matches(":popover-open")) node.showPopover();
    placePopover();
    window.addEventListener("scroll", placePopover, true);
    window.addEventListener("resize", placePopover);
    return () => {
      window.removeEventListener("scroll", placePopover, true);
      window.removeEventListener("resize", placePopover);
      if (node.matches(":popover-open")) node.hidePopover();
    };
  });

  </script>

<div class="relative {boxed ? 'w-full' : 'flex-1'}">
  {#if open && candidates.length > 0}
    <ul
      bind:this={listEl}
      popover="manual"
      class="max-h-56 overflow-y-auto rounded-lg border border-edge bg-panel px-0 py-1 shadow-lg"
    >
      {#each candidates as c, i (c.mention.kind + mentionToken(c.mention))}
        <li>
          <button
            type="button"
            class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm {i === index
              ? 'bg-neutral-700/60'
              : 'hover:bg-neutral-700/40'}"
            onmousedown={(e) => {
              e.preventDefault();
              choose(c);
            }}
          >
            <span class="truncate text-ink">{c.label}</span>
            <span
              class="ml-auto rounded px-1 text-[0.65rem] uppercase tracking-wide"
              style={chipStyle(chipColor(c.mention))}>{c.mention.kind}</span
            >
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if value === ""}
    <span
      class="pointer-events-none absolute flex text-sm text-ink-dim
        {boxed ? 'inset-y-0 items-start px-2.5 pt-2' : 'inset-y-0 items-center'}"
    >
      {placeholder}
    </span>
  {/if}

  <div
    bind:this={el}
    contenteditable="true"
    role="textbox"
    tabindex="0"
    aria-multiline="false"
    aria-label={placeholder}
    {oninput}
    {onkeydown}
    onkeyup={refresh}
    onclick={refresh}
    onblur={() => (open = false)}
    class="w-full overflow-y-auto whitespace-pre-wrap break-words text-sm text-ink focus:outline-none
      {boxed
        ? 'min-h-24 max-h-48 rounded-md border border-edge bg-neutral-800 px-2.5 py-2'
        : 'max-h-32 min-h-[1.25rem]'}"
  ></div>
</div>
