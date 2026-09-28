// A copy template from strings.ts with its `{name}` placeholders filled by rendered parts: the way a
// sentence the core words (e.g. `phaseAnnouncementKey`) shows names as chips instead of plain text.
// A placeholder with no part is left as written, so a missing part is visible rather than blank.
import type { ReactNode } from "react";
import { Fragment } from "react";

export function Template({ text, parts }: { text: string; parts: Record<string, ReactNode> }) {
  return (
    <>
      {text.split(/(\{\w+\})/).map((piece, i) => {
        const name = /^\{(\w+)\}$/.exec(piece)?.[1];
        return name !== undefined && name in parts ? <Fragment key={i}>{parts[name]}</Fragment> : piece;
      })}
    </>
  );
}
