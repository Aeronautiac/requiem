use crate::{
    Time,
    config::ability::{AbilityIdentifier, AbilityName},
};

const fn hrs(t: Time) -> Time {
    t * 60 * 60 * 1000
}

const fn mins(t: Time) -> Time {
    t * 60 * 1000
}

pub struct DefaultConfig {
    pub death_message: String,
    pub life_link_death_message: String,
    pub execution_death_message: String,
    pub pseudocide_duration: Time,                   // milliseconds
    pub universal_abilities: Vec<AbilityIdentifier>, // the abilities that everyone gets regardless
    // of role
    pub notebook_successes_per_day: u16,
    pub notebook_failures_per_day: u16,
    pub org_vote_time: Time,
    pub debate_default_timeout: Time,
    pub debate_shortened_timeout: Time,
    pub custody_timeout: Time,
    pub trial_vote_duration: Time,
    pub presentation_grace_timeout: Time,
    pub presentation_timeout: Time,
    pub kidnap_time: Time,
    pub autopsy_window: Time,
    pub autopsy_redaction: bool,
}

pub fn default_defaults() -> DefaultConfig {
    DefaultConfig {
        universal_abilities: vec![AbilityIdentifier {
            name: AbilityName::Contact,
            variant: 0,
        }],
        death_message: "They died from a sudden heart attack.".into(),
        life_link_death_message: "They died because of a life link.".into(),
        execution_death_message: "They were found guilty and subsequently executed.".into(),
        pseudocide_duration: hrs(24),
        notebook_successes_per_day: 1,
        notebook_failures_per_day: 3,
        org_vote_time: hrs(6),
        presentation_grace_timeout: hrs(1),
        presentation_timeout: mins(30),
        debate_default_timeout: hrs(1),
        debate_shortened_timeout: mins(15),
        custody_timeout: hrs(4),
        trial_vote_duration: hrs(6),
        kidnap_time: hrs(24),
        autopsy_window: hrs(6),
        autopsy_redaction: false,
    }
}
