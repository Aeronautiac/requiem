// The account panel: whoever this connection is, before any game is joined. Four states, one per
// AccountState kind — a guest sees a login/signup form, a member sees who they are.
import type { AccountPacket } from "amane-client/bindings.ts";
import type { Client } from "amane-client/client.ts";
import { execErrorText } from "amane-client/text.ts";
import type { FormEvent } from "react";
import { useState } from "react";
import { Button } from "../kit/Button.tsx";
import { FlashLine, useFlash } from "../kit/Flash.tsx";
import { Input } from "../kit/Input.tsx";
import { Modal } from "../kit/Modal.tsx";
import { tint } from "../style.ts";

export function AccountBox({ client }: { client: Client }) {
  const account = client.account;
  if (account.kind === "loading") {
    return <p className="text-sm text-ink-dim">…</p>;
  }
  if (account.kind === "unreachable") {
    return (
      <section className="flex items-center gap-2 border border-edge bg-panel px-3 py-2">
        <p className="min-w-0 flex-1 text-sm text-danger-text">{execErrorText(account.error)}</p>
        <Button size="sm" variant="ghost" onClick={() => void client.refresh_account()}>
          Retry
        </Button>
      </section>
    );
  }
  if (account.kind === "guest") return <GuestForm client={client} />;
  return <MemberInfo client={client} packet={account.packet} />;
}

function GuestForm({ client }: { client: Client }) {
  const [mode, setMode] = useState<"login" | "signup">("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const flash = useFlash();

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (username.trim() === "" || password === "") return;
    setBusy(true);
    const reply =
      mode === "login"
        ? await client.login(username.trim(), password)
        : await client.signup(username.trim(), password);
    setBusy(false);
    flash.reply(reply);
  }

  return (
    <section className="flex flex-col gap-3 border border-edge bg-panel p-3">
      <div className="flex gap-1 text-sm">
        <button
          type="button"
          className={`flex-1 border-b-2 py-1 ${mode === "login" ? "border-accent text-ink" : "border-transparent text-ink-dim"}`}
          onClick={() => setMode("login")}
        >
          Log in
        </button>
        <button
          type="button"
          className={`flex-1 border-b-2 py-1 ${mode === "signup" ? "border-accent text-ink" : "border-transparent text-ink-dim"}`}
          onClick={() => setMode("signup")}
        >
          Sign up
        </button>
      </div>
      <form className="flex flex-col gap-2" onSubmit={submit}>
        <Input
          placeholder="Username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoComplete="username"
        />
        <Input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete={mode === "login" ? "current-password" : "new-password"}
        />
        <Button type="submit" disabled={busy || username.trim() === "" || password === ""}>
          {mode === "login" ? "Log in" : "Sign up"}
        </Button>
        <FlashLine flash={flash} />
      </form>
    </section>
  );
}

function MemberInfo({ client, packet }: { client: Client; packet: AccountPacket }) {
  const [changingPassword, setChangingPassword] = useState(false);

  return (
    <section className="flex flex-col gap-2 border border-edge bg-panel p-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-sm text-ink">{packet.username}</p>
        {packet.role === "Admin" && (
          <span className="px-1.5 py-0.5 text-xs" style={tint("var(--color-accent)")}>
            admin
          </span>
        )}
        <span
          className="px-1.5 py-0.5 text-xs"
          style={tint(packet.verified ? "var(--color-ok-text)" : "var(--color-ink-dim)")}
        >
          {packet.verified ? "verified" : "unverified"}
        </span>
        <div className="ml-auto flex gap-2">
          <Button size="sm" variant="ghost" onClick={() => setChangingPassword(true)}>
            Change password
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void client.logout()}>
            Log out
          </Button>
        </div>
      </div>
      <ChangePasswordModal
        client={client}
        open={changingPassword}
        onClose={() => setChangingPassword(false)}
      />
    </section>
  );
}

function ChangePasswordModal({
  client,
  open,
  onClose,
}: {
  client: Client;
  open: boolean;
  onClose: () => void;
}) {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [busy, setBusy] = useState(false);
  const flash = useFlash();

  async function submit() {
    setBusy(true);
    const reply = await client.change_password(current, next);
    setBusy(false);
    if (flash.reply(reply, "Password changed.")) {
      setCurrent("");
      setNext("");
    }
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Change password"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button disabled={busy || current === "" || next === ""} onClick={() => void submit()}>
            Change
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-2">
        <Input
          type="password"
          placeholder="Current password"
          value={current}
          onChange={(e) => setCurrent(e.target.value)}
          autoComplete="current-password"
        />
        <Input
          type="password"
          placeholder="New password"
          value={next}
          onChange={(e) => setNext(e.target.value)}
          autoComplete="new-password"
        />
        <FlashLine flash={flash} />
      </div>
    </Modal>
  );
}
