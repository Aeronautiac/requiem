// Copying text for the user, the one way: success or failure always shows in the given flash line,
// so a copy never fails silently. `navigator.clipboard` is absent outside a secure context (a phone
// reaching the dev server over plain http), which lands in the catch like a refusal does.
import type { Flash } from "./Flash.tsx";

export async function copyToClipboard(text: string, flash: Flash, what = "Copied.") {
  try {
    await navigator.clipboard.writeText(text);
    flash.ok(what);
  } catch {
    flash.error("Couldn't copy here. Select the text and copy it by hand.");
  }
}
