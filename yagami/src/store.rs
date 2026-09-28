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
use std::time::Duration;

use lawliet_types::common::Time;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgConnection, PgPool, Row};

use crate::{
    account::{Account, AccountId, Role},
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

// one game's entry in the platform directory: what may be known about it without it running --
// that it exists, and how many keys (people) it holds.
pub struct DirectoryListing {
    pub game_id: GameId,
    pub keys: usize,
}

pub enum SignupRejected {
    UsernameTaken,
    InvalidUsername,
}

// everything a waking game needs to come back: its metadata and its full input log.
pub struct GameRecord {
    pub meta: GameMeta,
    pub inputs: Vec<VersionedInput>,
}

impl Store {
    // connect, then apply any not-yet-applied migrations in order (see sqlx::migrate).
    pub async fn connect(database_url: &str, pool_size: u32) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(pool_size)
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
    // anchored to now so the birth checkpoint (clock = 0, clock_wall = now) holds. the admin key is
    // saved to the owner in the same transaction, so an owned game never exists without it.
    pub async fn create_game(
        &self,
        initial_inputs: &[VersionedInput],
        owner: Option<AccountId>,
        admin_key: Option<&Key>,
    ) -> Result<GameId, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("INSERT INTO games (clock_wall, owner) VALUES ($1, $2) RETURNING id")
            .bind(wall_now())
            .bind(owner.map(|owner| owner as i64))
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
        if let (Some(owner), Some(admin_key)) = (owner, admin_key) {
            sqlx::query("INSERT INTO account_keys (account_id, game_id, key) VALUES ($1, $2, $3)")
                .bind(owner as i64)
                .bind(id)
                .bind(admin_key.as_str())
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
        write_meta(&mut tx, game_id, meta).await?;
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
        let mut tx = self.pool.begin().await?;
        write_meta(&mut tx, game_id, meta).await?;
        tx.commit().await?;
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

    // the platform directory: a listing for every active game, running or hibernated. the key
    // count is taken in SQL so no ledger leaves the store. a fresh row holds the '{}' default until
    // its first progress write, hence the array check.
    pub async fn game_directory(&self) -> Result<Vec<DirectoryListing>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id,
                    CASE WHEN jsonb_typeof(keys) = 'array' THEN jsonb_array_length(keys) ELSE 0 END
                        AS key_count
               FROM games WHERE status = 'active' ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut games = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let keys: i32 = row.try_get("key_count")?;
            games.push(DirectoryListing {
                game_id: id as GameId,
                keys: keys as usize,
            });
        }
        Ok(games)
    }

    // an active game's key ledger, without its log: enough to validate a key before waking it.
    // None if there is no such active game.
    pub async fn load_keys(
        &self,
        game_id: GameId,
    ) -> Result<Option<HashMap<Key, Privileges>>, sqlx::Error> {
        let keys_json: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT keys FROM games WHERE id = $1 AND status = 'active'")
                .bind(game_id as i64)
                .fetch_optional(&self.pool)
                .await?;
        Ok(keys_json.map(|json| keys_from_json(&json)))
    }

    // an active game's metadata and full input log -- what a waking game replays to come back up.
    // None if there is no such active game (e.g. it was ended while its wake was in flight).
    pub async fn load_game(&self, game_id: GameId) -> Result<Option<GameRecord>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT last_reached, clock, clock_wall, keys FROM games
              WHERE id = $1 AND status = 'active'",
        )
        .bind(game_id as i64)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let last_reached: i64 = row.try_get("last_reached")?;
        let clock: i64 = row.try_get("clock")?;
        let clock_wall: i64 = row.try_get("clock_wall")?;
        let keys_json: serde_json::Value = row.try_get("keys")?;
        Ok(Some(GameRecord {
            meta: GameMeta {
                last_reached: last_reached as Time,
                clock: clock as Time,
                clock_wall,
                keys: keys_from_json(&keys_json),
            },
            inputs: self.load_inputs(game_id).await?,
        }))
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

    // mark a game ended so nothing can wake it again, and drop every key saved for it (the row
    // stays, so the FK cascade never fires). false if there was no such active game.
    pub async fn end_game(&self, game_id: GameId) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result =
            sqlx::query("UPDATE games SET status = 'ended' WHERE id = $1 AND status = 'active'")
                .bind(game_id as i64)
                .execute(&mut *tx)
                .await?;
        sqlx::query("DELETE FROM account_keys WHERE game_id = $1")
            .bind(game_id as i64)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }

    // claim a username by inserting it. the table is the arbiter of both rules: its CHECK is the
    // one definition of a valid username, and the unique index on lower(username) means two
    // concurrent signups can't both win.
    pub async fn create_account(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<Result<AccountId, SignupRejected>, sqlx::Error> {
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO accounts (username, password_hash) VALUES ($1, $2) RETURNING id",
        )
        .bind(username)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await;
        match inserted {
            Ok(id) => Ok(Ok(id as AccountId)),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                Ok(Err(SignupRejected::UsernameTaken))
            }
            Err(sqlx::Error::Database(e)) if e.is_check_violation() => {
                Ok(Err(SignupRejected::InvalidUsername))
            }
            Err(e) => Err(e),
        }
    }

    // what a login checks a password against. usernames match ignoring case.
    pub async fn load_credentials(
        &self,
        username: &str,
    ) -> Result<Option<(AccountId, String)>, sqlx::Error> {
        let row =
            sqlx::query("SELECT id, password_hash FROM accounts WHERE lower(username) = lower($1)")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        row.map(|row| {
            let id: i64 = row.try_get("id")?;
            Ok((id as AccountId, row.try_get("password_hash")?))
        })
        .transpose()
    }

    // mint a session, first reclaiming every expired one. an expired session is already dead to
    // session_account, so this is only housekeeping, done where sessions are made.
    pub async fn create_session(
        &self,
        token_hash: &[u8; 32],
        account_id: AccountId,
        ttl: Duration,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "INSERT INTO sessions (token_hash, account_id, expires_at)
             VALUES ($1, $2, now() + make_interval(secs => $3))",
        )
        .bind(&token_hash[..])
        .bind(account_id as i64)
        .bind(ttl.as_secs_f64())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // resolve a session to its account and push its expiry out by `ttl` (sliding), in one
    // statement. None if there is no such session or it has expired.
    pub async fn session_account(
        &self,
        token_hash: &[u8; 32],
        ttl: Duration,
    ) -> Result<Option<Account>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE sessions SET expires_at = now() + make_interval(secs => $2)
               FROM accounts
              WHERE sessions.token_hash = $1
                AND sessions.expires_at > now()
                AND accounts.id = sessions.account_id
          RETURNING accounts.id, accounts.username, accounts.role, accounts.verified",
        )
        .bind(&token_hash[..])
        .bind(ttl.as_secs_f64())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let id: i64 = row.try_get("id")?;
            let role: String = row.try_get("role")?;
            Ok(Account {
                id: id as AccountId,
                username: row.try_get("username")?,
                role: Role::from_db(&role).ok_or_else(|| {
                    sqlx::Error::Decode(format!("unknown account role {role:?}").into())
                })?,
                verified: row.try_get("verified")?,
            })
        })
        .transpose()
    }

    pub async fn delete_session(&self, token_hash: &[u8; 32]) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(&token_hash[..])
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // the active games an account owns: what the per-account quota counts.
    pub async fn owned_games(&self, owner: AccountId) -> Result<Vec<GameId>, sqlx::Error> {
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM games WHERE owner = $1 AND status = 'active' ORDER BY id",
        )
        .bind(owner as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(ids.into_iter().map(|id| id as GameId).collect())
    }

    // every key an account has saved, with what each permits. all of them are live: a key leaves
    // this table when its game's key set drops it or the game ends.
    //
    // what a key permits is read from the game row's stored ledger, which is written in the same
    // transaction as every accepted input, so it is current whether the game is awake or not.
    pub async fn saved_keys(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<(GameId, Key, PrivilegeSet)>, sqlx::Error> {
        let rows: Vec<(i64, String, serde_json::Value)> = sqlx::query_as(
            "SELECT a.game_id, a.key, g.keys
               FROM account_keys a JOIN games g ON g.id = a.game_id
              WHERE a.account_id = $1 AND g.status = 'active'
              ORDER BY a.game_id",
        )
        .bind(account_id as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(game_id, key, ledger)| {
                // the prune shares write_meta's transaction, so a saved key is always in its game's
                // ledger; a key that somehow isn't is skipped rather than shown with no powers.
                let ledger: Vec<(Key, PrivilegeSet)> = serde_json::from_value(ledger).ok()?;
                let (key, privileges) = ledger.into_iter().find(|(k, _)| k.as_str() == key)?;
                Some((game_id as GameId, key, privileges))
            })
            .collect())
    }

    pub async fn forget_key(
        &self,
        account_id: AccountId,
        game_id: GameId,
        key: &Key,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM account_keys WHERE account_id = $1 AND game_id = $2 AND key = $3")
            .bind(account_id as i64)
            .bind(game_id as i64)
            .bind(key.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // replace the password and end every session but `keep`, in one transaction.
    pub async fn change_password(
        &self,
        account_id: AccountId,
        password_hash: &str,
        keep: &[u8; 32],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE accounts SET password_hash = $2 WHERE id = $1")
            .bind(account_id as i64)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE account_id = $1 AND token_hash <> $2")
            .bind(account_id as i64)
            .bind(&keep[..])
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // an active game's owner. None if there is no such active game; Some(None) if it has no owner.
    pub async fn game_owner(
        &self,
        game_id: GameId,
    ) -> Result<Option<Option<AccountId>>, sqlx::Error> {
        let owner: Option<Option<i64>> =
            sqlx::query_scalar("SELECT owner FROM games WHERE id = $1 AND status = 'active'")
                .bind(game_id as i64)
                .fetch_optional(&self.pool)
                .await?;
        Ok(owner.map(|owner| owner.map(|owner| owner as AccountId)))
    }

    // save a key under an account, if its game is still active and still holds it. saving it twice
    // is a no-op.
    //
    // FOR SHARE conflicts with the UPDATE games that both end_game and write_meta make before they
    // prune saved keys, so the two serialize: if the prune commits first, the check here reads the
    // row it left and skips the insert; if the insert commits first, the prune runs after and sees
    // it. either way a saved key can't outlive the key set it was checked against.
    pub async fn save_key(
        &self,
        account_id: AccountId,
        game_id: GameId,
        key: &Key,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let keys_json: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT keys FROM games WHERE id = $1 AND status = 'active' FOR SHARE",
        )
        .bind(game_id as i64)
        .fetch_optional(&mut *tx)
        .await?;
        if keys_json.is_some_and(|json| keys_from_json(&json).contains_key(key)) {
            sqlx::query(
                "INSERT INTO account_keys (account_id, game_id, key) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(account_id as i64)
            .bind(game_id as i64)
            .bind(key.as_str())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

// write a game's progress into its row, then drop any saved key the new key set no longer holds
// (revoked, or erased by a rewind). must share the caller's transaction, so a saved key never
// outlives the key set it was checked against.
async fn write_meta(
    conn: &mut PgConnection,
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
    .execute(&mut *conn)
    .await?;
    let live: Vec<&str> = meta.keys.keys().map(Key::as_str).collect();
    sqlx::query("DELETE FROM account_keys WHERE game_id = $1 AND key <> ALL($2)")
        .bind(game_id as i64)
        .bind(live)
        .execute(&mut *conn)
        .await?;
    Ok(())
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

