-- Game persistence for yagami.
--
--   games   -- one row per game; only metadata that CANNOT be derived by replaying the
--             input log (last_reached, the sandbox clock checkpoint, the keys cache for
--             on-demand boot).
--   inputs  -- the append-only ACCEPTED input stream; THE source of truth for game state.
--             State is a RAM projection rebuilt by replaying a game's inputs in seq order.
--   crashes -- a GLOBAL debug log (not per-game) of engine crash records. Each row stores the
--             accepted-input SEQUENCE that led up to the crash (including the crashing input as
--             its last element), so a crash is fully reproducible. They are inert -- never
--             replayed -- and survive rewind (they live in their own table, so a truncate of
--             `inputs` never touches them). game_id is a context column, deliberately not a
--             foreign key: a crash record outlives whatever game it happened in.
--
-- Write-ahead: an input row is committed BEFORE the client is acknowledged, and every append
-- is idempotent (UNIQUE on (game_id, seq)), so a retry can never double-apply.

CREATE TABLE games (
    id           BIGSERIAL PRIMARY KEY,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- 'active' while live, 'ended' once torn down. resume skips ended games.
    status       TEXT NOT NULL DEFAULT 'active',
    -- the latest game-time the engine reached; not derivable without replaying every input,
    -- so it is cached here for a cheap boot / on-demand resume.
    last_reached BIGINT NOT NULL DEFAULT 0,
    -- the sandbox clock's CURRENT VIRTUAL TIME (GameClock::now()) and the wall-clock millis it
    -- was true at. start is per-process and useless to store; anchor alone ignores the elapsed
    -- baked into it. storing now() + wall makes the clock real-time-continuous across a restart:
    -- on resume we add the downtime (current_wall - clock_wall) so virtual time keeps tracking
    -- real time instead of pausing while the server is down.
    clock        BIGINT NOT NULL DEFAULT 0,
    clock_wall   BIGINT NOT NULL DEFAULT 0,
    -- the reconstructed key set (KeyRoster), stored so the server can serve get_ticket (key
    -- validation) WITHOUT booting the engine -- on-demand boot, not immediate.
    keys         JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE inputs (
    game_id BIGINT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    -- per-game monotonic append index; the idempotency key for write-ahead.
    seq     BIGINT NOT NULL,
    -- the accepted ServerInput (action or sim control), serialized.
    input   JSONB NOT NULL,
    PRIMARY KEY (game_id, seq)
);

CREATE TABLE crashes (
    id      BIGSERIAL PRIMARY KEY,
    game_id BIGINT,                        -- context only; not a FK on purpose
    at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- the accepted-input sequence leading up to the crash, in order, with the crashing input
    -- as the last element. replaying this reproduces the crash.
    seq     JSONB NOT NULL
);
