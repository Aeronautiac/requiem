-- Accounts replace platform keys as the authority over PLATFORM actions (creating, ending games).
-- In-game power is untouched: it still comes only from a game's own keys.
--
--   accounts     -- one row per signup. `verified` is set by any verification method (by hand in
--                   the DB for now); whether creating a game requires it is an env switch.
--   sessions     -- a login. the client holds a random token; only its sha256 is stored, so a
--                   leaked table cannot be replayed as logins.
--   account_keys -- game keys an account has saved, so a player gets back into their games from
--                   any device. a saved key never outlives the game's key set: the store prunes
--                   it in the same transaction that writes games.keys, and when the game ends.

DROP TABLE platform_keys;

CREATE TABLE accounts (
    id            BIGSERIAL PRIMARY KEY,
    -- displayed as typed, unique ignoring case (the index below).
    username      TEXT NOT NULL CHECK (username ~ '^[A-Za-z0-9_]{3,20}$'),
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    verified      BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX accounts_username_key ON accounts (lower(username));

CREATE TABLE sessions (
    token_hash BYTEA PRIMARY KEY,
    account_id BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_account_id ON sessions (account_id);
-- every login first reclaims the expired sessions.
CREATE INDEX sessions_expires_at ON sessions (expires_at);

-- NULL owner: a game from before accounts, or whose owner was deleted. only an admin manages it.
ALTER TABLE games ADD COLUMN owner BIGINT REFERENCES accounts(id) ON DELETE SET NULL;

CREATE INDEX games_owner ON games (owner);

CREATE TABLE account_keys (
    account_id BIGINT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    game_id    BIGINT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    PRIMARY KEY (account_id, game_id, key)
);

-- the prune deletes by game; the primary key leads with account_id and can't serve it.
CREATE INDEX account_keys_game_id ON account_keys (game_id);
