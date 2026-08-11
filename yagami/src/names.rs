// The reservoir the server draws people's names from.
//
// First and last are kept apart so N of each yields N*N people rather than N. Both lists are plain
// text, one name per line, embedded at compile time -- editing them is not a code change, and there
// is no file for a deployment to be missing.
//
// ASCII letters only, no punctuation or accents: a true name is compared for equality and typed by
// players from memory, and neither survives a stray apostrophe.
//
// Nothing here is replayable and nothing needs to be. A drawn name is written into the action before
// the engine ever sees it, so the log carries the name itself and a replay reproduces it exactly --
// which is the whole reason the name is chosen here rather than asked for mid-action.
//
// The pool owns the set of names in play: it is told about names the world holds (admin-chosen, or
// one that arrives on a replayed command) and a draw never hands out one of them again.

use std::collections::HashSet;

const FIRST: &str = include_str!("../names/first.txt");
const LAST: &str = include_str!("../names/last.txt");

// How many random pairs to try before giving up on luck and sweeping. Far more than a game with a
// full lobby will ever need: a collision requires drawing one of the handful of names already in
// play out of a space of millions.
const RANDOM_ATTEMPTS: usize = 32;

pub struct NamePool {
    first: Vec<&'static str>,
    last: Vec<&'static str>,
    // every name the world already holds, so a draw and the engine's uniqueness check agree.
    taken: HashSet<String>,
}

fn parse(list: &'static str) -> Vec<&'static str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

impl NamePool {
    pub fn new() -> Self {
        let pool = NamePool {
            first: parse(FIRST),
            last: parse(LAST),
            taken: HashSet::new(),
        };
        // An empty list is a broken build, not a runtime condition -- the files are embedded, so
        // this cannot become true after the binary exists.
        assert!(
            !pool.first.is_empty() && !pool.last.is_empty(),
            "name reservoir is empty: names/first.txt and names/last.txt must each hold at least one name"
        );
        pool
    }

    // Record a name the world already holds, so a later draw never hands it out again. Called with
    // a name that reached the engine through another route -- an admin wrote it, or it came back on
    // a replayed command -- so the pool stays in step with the engine's own uniqueness check.
    pub fn mark_taken(&mut self, name: &str) {
        self.taken.insert(name.to_string());
    }

    // A full, previously-unhanded name, or None when every pair in the reservoir is spoken for.
    //
    // A successful draw is marked taken before it is returned, so consecutive draws never repeat.
    // Tries at random first because that is what makes names unpredictable; the sweep afterwards is
    // only there so a genuinely exhausted pool returns an answer instead of looping.
    pub fn draw(&mut self) -> Option<String> {
        for _ in 0..RANDOM_ATTEMPTS {
            let candidate = format!(
                "{} {}",
                self.first[self.index(self.first.len())],
                self.last[self.index(self.last.len())]
            );
            if !self.taken.contains(&candidate) {
                self.taken.insert(candidate.clone());
                return Some(candidate);
            }
        }

        for first in &self.first {
            for last in &self.last {
                let candidate = format!("{first} {last}");
                if !self.taken.contains(&candidate) {
                    self.taken.insert(candidate.clone());
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn index(&self, len: usize) -> usize {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).expect("OS CSPRNG unavailable");
        (u64::from_le_bytes(bytes) % len as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_lists_are_ascii_letters_only() {
        let pool = NamePool::new();
        for name in pool.first.iter().chain(pool.last.iter()) {
            assert!(
                name.chars().all(|c| c.is_ascii_alphabetic()),
                "{name} is not plain ASCII letters"
            );
        }
    }

    #[test]
    fn consecutive_draws_never_repeat() {
        let mut pool = NamePool::new();
        let first = pool.draw().expect("a fresh pool always has one");
        let second = pool.draw().expect("and a second");
        assert_ne!(first, second);
    }

    #[test]
    fn a_draw_avoids_marked_names() {
        let mut pool = NamePool::new();
        pool.mark_taken("Robyn Holmes");
        for _ in 0..200 {
            let name = pool.draw().expect("a large pool stays drawable");
            assert_ne!(name, "Robyn Holmes");
        }
    }

    // The sweep is the only thing standing between an exhausted pool and a loop.
    #[test]
    fn an_exhausted_pool_gives_up() {
        let mut pool = NamePool::new();
        let all: Vec<String> = pool
            .first
            .iter()
            .flat_map(|f| pool.last.iter().map(move |l| format!("{f} {l}")))
            .collect();
        for name in &all {
            pool.mark_taken(name);
        }
        assert_eq!(pool.draw(), None);
    }
}
