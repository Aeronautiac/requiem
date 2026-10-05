// TODO:
// implement dynamic rules - the game should alter its behaviour based on whats in the ruleset

#[allow(dead_code)]
pub struct Ruleset {
    game_continues_after_l_death: bool,
    kira_requires_kills: bool,
    autopsy_blurs_names: bool,
}
