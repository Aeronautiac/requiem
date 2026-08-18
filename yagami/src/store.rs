// Postgres persistence: the accepted input log (the source of truth), per-game metadata, and a
// global crash log.
//
// This is the only module that knows SQL. Everything upstream of it (game.rs) just asks for
// "append this input, write-ahead", "load a game's inputs", "persist progress", or "record a
// crash" and never sees a query.
//
// The three tables (see migrations/0001_init.sql):
//   games   -- one row per game; metadata that cannot be derived by replay (last_reached, the
//             sandbox clock checkpoint, the keys cache for on-demand boot).
//   inputs  -- the append-only accepted stream, serialized to jsonb, keyed by (game_id, seq).
//   crashes -- a global debug log of crash reproduction sequences (inert, never replayed).
//
// Write-ahead guarantee: an input row is committed (in the same transaction as the metadata
// update) BEFORE the game task acknowledges the client. The (game_id, seq) primary key makes the
// append idempotent, so a retried write can never double-apply.

use std::collections::HashMap;

use lawliet_types::common::Time;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::{
    auth::{Key, Privileges, to_flags},
    state::GameId,
    wire::{VersionedInput, privileges_to_wire},
};
use yagami_wire::PrivilegeSet;

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

// the parts of a game's progress that are cheap to persist and not derivable from the log alone.
// keys is a cache so a restarted server can validate get_ticket without booting the engine.
#[derive(Default, Clone)]
pub struct GameMeta {
    pub last_reached: Time,
    // the sandbox clock's current virtual time, and the wall-clock millis it was true at.
    // together these make the clock real-time-continuous across a restart.
    pub clock: Time,
    pub clock_wall: i64, // epoch millis; BIGINT column (sqlx has no i128 binding for it)
    pub keys: HashMap<Key, Privileges>,
}

// everything a restarted server needs to bring a game back: its metadata and its full input log.
pub struct GameRecord {
    pub id: GameId,
    pub meta: GameMeta,
    pub inputs: Vec<VersionedInput>,
}

impl Store {
    // connect, then apply any not-yet-applied migrations in order (see sqlx::migrate).
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    // create a game's durable record: a fresh game task calls this AFTER its first boot succeeds,
    // so a game that fails to boot is never written (and the BIGSERIAL sequence never advances --
    // no gaps, no pre-write). the row and the game's initial accepted stream (`initial_inputs`:
    // the task-generated InitializeEngine + the admin-key creation) are written in one transaction
    // -- the one place the initial stream is inserted as a group, because the normal append_input
    // path would imply the row already exists, which it does not until after boot. clock_wall is
    // anchored to now so the birth checkpoint (clock = 0, clock_wall = now) holds.
    pub async fn create_game(
        &self,
        initial_inputs: &[VersionedInput],
    ) -> Result<GameId, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("INSERT INTO games (clock_wall) VALUES ($1) RETURNING id")
            .bind(wall_now())
            .fetch_one(&mut *tx)
            .await?;
        let id: i64 = row.try_get("id")?;
        for (i, input) in initial_inputs.iter().enumerate() {
            let input_json = serde_json::to_value(input).map_err(json_err)?;
            sqlx::query("INSERT INTO inputs (game_id, seq, input) VALUES ($1, $2, $3)")
                .bind(id)
                .bind(i as i64)
                .bind(input_json)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(id as GameId)
    }

    // WRITE-AHEAD append: insert one accepted input (idempotently) and fold the game's latest
    // progress into its metadata row, atomically. Callers must await this before acknowledging.
    pub async fn append_input(
        &self,
        game_id: GameId,
        seq: i64,
        input: &VersionedInput,
        meta: &GameMeta,
    ) -> Result<(), sqlx::Error> {
        let input_json = serde_json::to_value(input).map_err(json_err)?;
        let keys_json = serde_json::to_value(keys_to_vec(&meta.keys)).map_err(json_err)?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO inputs (game_id, seq, input) VALUES ($1, $2, $3)
             ON CONFLICT (game_id, seq) DO NOTHING",
        )
        .bind(game_id as i64)
        .bind(seq)
        .bind(input_json)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE games
                SET last_reached = $2, clock = $3, clock_wall = $4, keys = $5
              WHERE id = $1",
        )
        .bind(game_id as i64)
        .bind(meta.last_reached as i64)
        .bind(meta.clock as i64)
        .bind(meta.clock_wall)
        .bind(keys_json)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // update only the metadata row (no new input). used after a rewind-truncate + rebuild, and
    // after a boot, to keep the games row current with what the replay reconstructed.
    pub async fn persist_progress(
        &self,
        game_id: GameId,
        meta: &GameMeta,
    ) -> Result<(), sqlx::Error> {
        let keys_json = serde_json::to_value(keys_to_vec(&meta.keys)).map_err(json_err)?;
        sqlx::query(
            "UPDATE games
                SET last_reached = $2, clock = $3, clock_wall = $4, keys = $5
              WHERE id = $1",
        )
        .bind(game_id as i64)
        .bind(meta.last_reached as i64)
        .bind(meta.clock as i64)
        .bind(meta.clock_wall)
        .bind(keys_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // load one game's full accepted stream, in append order.
    pub async fn load_inputs(&self, game_id: GameId) -> Result<Vec<VersionedInput>, sqlx::Error> {
        let rows = sqlx::query("SELECT input FROM inputs WHERE game_id = $1 ORDER BY seq")
            .bind(game_id as i64)
            .fetch_all(&self.pool)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let value: serde_json::Value = row.try_get("input")?;
            let input: VersionedInput = serde_json::from_value(value).map_err(json_err)?;
            out.push(input);
        }
        Ok(out)
    }

    // every active game and its inputs -- what a restarted server replays to come back up.
    pub async fn resume(&self) -> Result<Vec<GameRecord>, sqlx::Error> {
        let rows =
            sqlx::query("SELECT id, last_reached, clock, clock_wall, keys FROM games WHERE status = 'active' ORDER BY id")
                .fetch_all(&self.pool)
                .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let last_reached: i64 = row.try_get("last_reached")?;
            let clock: i64 = row.try_get("clock")?;
            let clock_wall: i64 = row.try_get("clock_wall")?;
            let keys_json: serde_json::Value = row.try_get("keys")?;
            let keys = keys_from_json(&keys_json);

            let inputs = self.load_inputs(id as GameId).await?;
            records.push(GameRecord {
                id: id as GameId,
                meta: GameMeta {
                    last_reached: last_reached as Time,
                    clock: clock as Time,
                    clock_wall,
                    keys,
                },
                inputs,
            });
        }
        Ok(records)
    }

    // remove the tail of a game's log from `from_seq` onward -- the durable half of a
    // backward time-travel truncate. crash records are untouched (separate table).
    pub async fn delete_inputs_from(
        &self,
        game_id: GameId,
        from_seq: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM inputs WHERE game_id = $1 AND seq >= $2")
            .bind(game_id as i64)
            .bind(from_seq)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // record a crash: the accepted-input sequence leading up to it (the crashing input is the
    // last element), as a global, inert debug row.
    pub async fn record_crash(
        &self,
        game_id: GameId,
        sequence: &[VersionedInput],
    ) -> Result<(), sqlx::Error> {
        let seq_json = serde_json::to_value(sequence).map_err(json_err)?;
        sqlx::query("INSERT INTO crashes (game_id, seq) VALUES ($1, $2)")
            .bind(game_id as i64)
            .bind(seq_json)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // mark a game ended so a restart does not try to resume it.
    pub async fn end_game(&self, game_id: GameId) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE games SET status = 'ended' WHERE id = $1")
            .bind(game_id as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // is this a platform admin key (the allowlist that may create/end games)?
    pub async fn is_platform_admin(&self, key: &str) -> Result<bool, sqlx::Error> {
        let found: Option<i32> =
            sqlx::query_scalar("SELECT 1 FROM platform_keys WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(found.is_some())
    }
}

fn json_err(e: serde_json::Error) -> sqlx::Error {
    // the shapes we serialize (VersionedInput, the key ledger) are all serializable by construction;
    // a failure here is a bug, surfaced as a protocol-level error rather than a panic.
    sqlx::Error::Protocol(format!("json serialization failed: {e}"))
}

// current wall-clock time in epoch millis. the sandbox clock checkpoint stores the virtual time
// alongside the wall time it was true at, so a resume can add the downtime.
pub(crate) fn wall_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("wall clock before epoch")
        .as_millis() as i64
}

// keys are stored as the wire PrivilegeSet (actors + capability names), which is the serde shape.
fn keys_to_vec(keys: &HashMap<Key, Privileges>) -> Vec<(Key, PrivilegeSet)> {
    keys.iter()
        .map(|(k, p): (&Key, &Privileges)| (k.clone(), privileges_to_wire(p)))
        .collect()
}

fn keys_from_json(value: &serde_json::Value) -> HashMap<Key, Privileges> {
    serde_json::from_value::<Vec<(Key, PrivilegeSet)>>(value.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|(k, ps)| {
            (
                k,
                Privileges {
                    actors: ps.actors,
                    capabilities: to_flags(&ps.capabilities),
                },
            )
        })
        .collect()
}