-- platform admin keys: the allowlist of keys that may create/end games. managed directly in the
-- DB (e.g. via a UI like pgAdmin/adminer) -- no allowlist file, no hardcoded `true`.
CREATE TABLE platform_keys (
    key         TEXT PRIMARY KEY,
    description TEXT NOT NULL
);
