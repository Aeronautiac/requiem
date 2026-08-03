<script lang="ts">
  // The message composer with @-mention autocomplete AND live chips. A contenteditable rather than
  // an <input>, because a mention has to sit in the text as one atomic, non-editable pill: typing
  // `@` opens a narrowing list of players/roles/orgs/System, and choosing one drops a chip in place
  // of the `@query`. The editable content is the source of truth; `value` is the token string it
  // serialises to (`@<player:3:0>` …), which is what actually gets sent and what MentionText later
  // re-parses back into chips.
  import { ROLES } from "../../constants";
  import type { Mention } from "../../game/helpers.svelte";
  import {
    chipStyle,
    mentionColorVar,
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
    placeholder?: string;
    onsubmit: () => void;
  }
  let { value = $bindable(), players, orgs, placeholder, onsubmit }: Props = $props();

  type Candidate = { mention: Mention; label: string };

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
    chip.style.cssText = chipStyle(mentionColorVar(c.mention));
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
</script>

<div class="relative flex-1">
  {#if open && candidates.length > 0}
    <ul
      class="absolute bottom-full left-0 z-10 mb-2 max-h-56 w-72 overflow-y-auto rounded-lg border border-edge bg-panel py-1 shadow-lg"
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
              style={chipStyle(mentionColorVar(c.mention))}>{c.mention.kind}</span
            >
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if value === ""}
    <span
      class="pointer-events-none absolute inset-y-0 left-0 flex items-center text-sm text-ink-dim"
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
    class="max-h-32 min-h-[1.25rem] w-full overflow-y-auto whitespace-pre-wrap break-words text-sm text-ink focus:outline-none"
  ></div>
</div>
