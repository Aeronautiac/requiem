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

const FIRST: &str = include_str!("../names/first.txt");
const LAST: &str = include_str!("../names/last.txt");

// How many random pairs to try before giving up on luck and sweeping. Far more than a game with a
// full lobby will ever need: a collision requires drawing one of the handful of names already in
// play out of a space of millions.
const RANDOM_ATTEMPTS: usize = 32;

pub struct NamePool {
    first: Vec<&'static str>,
    last: Vec<&'static str>,
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
        };
        // An empty list is a broken build, not a runtime condition -- the files are embedded, so
        // this cannot become true after the binary exists.
        assert!(
            !pool.first.is_empty() && !pool.last.is_empty(),
            "name reservoir is empty: names/first.txt and names/last.txt must each hold at least one name"
        );
        pool
    }

    // A full name that `is_taken` rejects, or None when every pair in the reservoir is spoken for.
    //
    // Tries at random first because that is what makes names unpredictable; the sweep afterwards is
    // only there so a genuinely exhausted pool returns an answer instead of looping.
    pub fn draw(&self, is_taken: impl Fn(&str) -> bool) -> Option<String> {
        for _ in 0..RANDOM_ATTEMPTS {
            let candidate = format!(
                "{} {}",
                self.first[self.index(self.first.len())],
                self.last[self.index(self.last.len())]
            );
            if !is_taken(&candidate) {
                return Some(candidate);
            }
        }

        for first in &self.first {
            for last in &self.last {
                let candidate = format!("{first} {last}");
                if !is_taken(&candidate) {
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
    fn a_draw_avoids_taken_names() {
        let pool = NamePool::new();
        let first = pool.draw(|_| false).expect("a fresh pool always has one");
        let second = pool.draw(|name| name == first).expect("and a second");
        assert_ne!(first, second);
    }

    // The sweep is the only thing standing between an exhausted pool and a loop.
    #[test]
    fn an_exhausted_pool_gives_up() {
        let pool = NamePool::new();
        assert_eq!(pool.draw(|_| true), None);
    }
}
