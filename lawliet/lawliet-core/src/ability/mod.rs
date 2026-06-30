pub use lawliet_types::ability::{AbilityBehaviour, AbilityName};

use crate::{
    action::{ActionActor, ActionContext, ActionError},
    chargepool::{PoolLink, PoolLinkType},
    common::{AbilityKey, ChargeCount, ChargePoolKey, LinkWeight, Variant},
    engine::Engine,
    ownership::OwnershipStruct,
};
use indexmap::IndexSet;

pub mod anonymous_announcement;
pub mod anonymous_contact;
pub mod anonymous_kidnap;
pub mod anonymous_prosecute;
pub mod autopsy;
pub mod blackout;
pub mod bug;
pub mod civilian_arrest;
pub mod contact;
pub mod fake_lounge;
pub mod false_anonymous_contact;
pub mod gun;
pub mod ipp;
pub mod pseudocide;
pub mod public_kidnap;
pub mod shinigami_sacrifice;
pub mod tap_in;
pub mod true_name_reroll;
pub mod under_the_radar;
pub mod unlawful_arrest;

pub trait AbilityInterface {
    fn ability_name(&self) -> AbilityName;
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        ability: AbilityKey,
        version: u8,
        mutate: bool,
    ) -> AbilityResult;
}

impl AbilityInterface for AbilityBehaviour {
    fn ability_name(&self) -> AbilityName {
        match self {
            AbilityBehaviour::Contact(a) => a.ability_name(),
            AbilityBehaviour::Pseudocide(a) => a.ability_name(),
            AbilityBehaviour::Gun(a) => a.ability_name(),
            AbilityBehaviour::AnonymousAnnouncement(a) => a.ability_name(),
            AbilityBehaviour::AnonymousContact(a) => a.ability_name(),
            AbilityBehaviour::AnonymousKidnap(a) => a.ability_name(),
            AbilityBehaviour::PublicKidnap(a) => a.ability_name(),
            AbilityBehaviour::AnonymousProsecute(a) => a.ability_name(),
            AbilityBehaviour::Autopsy(a) => a.ability_name(),
        }
    }

    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        ability: AbilityKey,
        version: u8,
        mutate: bool,
    ) -> AbilityResult {
        match self {
            AbilityBehaviour::Contact(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::Pseudocide(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::Gun(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::AnonymousAnnouncement(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::AnonymousContact(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::AnonymousKidnap(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::PublicKidnap(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::AnonymousProsecute(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::Autopsy(a) => a.handle(eng, ctx, actor, ability, version, mutate),
        }
    }
}

// a volatile link is destroyed when the owner changes
#[derive(Hash, PartialEq, PartialOrd, Eq, Ord, Debug, Clone)]
pub struct AbilityPoolLink {
    pub volatile: bool,
    pub link: PoolLink,
}

pub type AbilityResult = Result<(), ActionError>;

#[derive(Debug)]
pub struct Ability {
    pub ownership_struct: OwnershipStruct,
    pub pool_links: IndexSet<AbilityPoolLink>,
    pub ability_name: AbilityName,
    pub variant: Variant,
}

impl Ability {
    pub fn new(ability_name: AbilityName, variant: Variant, transferrable: bool) -> Self {
        Ability {
            pool_links: IndexSet::new(),
            variant,
            ability_name,
            ownership_struct: OwnershipStruct::new(transferrable),
        }
    }

    pub fn add_link(
        &mut self,
        link_dest: ChargePoolKey,
        link_type: PoolLinkType,
        weight: LinkWeight,
        volatile: bool,
    ) -> Option<AbilityPoolLink> {
        let removed: Option<AbilityPoolLink> = self
            .pool_links
            .extract_if(.., |l| l.link.link_dest == link_dest)
            .next();
        self.pool_links.insert(AbilityPoolLink {
            volatile,
            link: PoolLink {
                link_type,
                link_dest,
                weight,
            },
        });
        removed
    }

    pub fn remove_link(&mut self, link_dest: ChargePoolKey) {
        self.pool_links.retain(|l| l.link.link_dest != link_dest)
    }

    pub fn get_usage_limit(&self, eng: &Engine) -> Option<ChargeCount> {
        let mut lowest_limit: Option<ChargeCount> = None;
        let mut highest_permissive: Option<ChargeCount> = None;

        for link_container in self.pool_links.iter() {
            let pool = eng
                .world
                .get_charge_pool(link_container.link.link_dest)
                .expect("expected valid link destination");
            let limit = pool.charges / link_container.link.weight;
            match link_container.link.link_type {
                PoolLinkType::Limit => {
                    if let Some(lowest) = lowest_limit
                        && limit < lowest
                    {
                        lowest_limit = Some(limit);
                    } else if lowest_limit.is_none() {
                        lowest_limit = Some(limit);
                    }
                }
                PoolLinkType::Pool => {
                    if let Some(highest) = highest_permissive
                        && limit > highest
                    {
                        highest_permissive = Some(limit);
                    } else if highest_permissive.is_none() {
                        highest_permissive = Some(limit);
                    }
                }
            }
        }

        lowest_limit.or(highest_permissive)
    }

    // usages_remaining: same constraining-pool logic as get_usage_limit.
    // iterations_to_reset: minimum across all pools (soonest reset), independent of usage constraint.
    pub fn get_ability_view_counts(
        &self,
        eng: &Engine,
    ) -> (ChargeCount, crate::common::IterationCount) {
        let mut lowest_limit: Option<ChargeCount> = None;
        let mut highest_permissive: Option<ChargeCount> = None;
        let mut min_reset: Option<crate::common::IterationCount> = None;

        for link_container in self.pool_links.iter() {
            let pool = eng
                .world
                .get_charge_pool(link_container.link.link_dest)
                .expect("expected valid link destination");
            let usages = pool.charges / link_container.link.weight;
            match link_container.link.link_type {
                PoolLinkType::Limit => {
                    if lowest_limit.map_or(true, |l| usages < l) {
                        lowest_limit = Some(usages);
                    }
                }
                PoolLinkType::Pool => {
                    if highest_permissive.map_or(true, |h| usages > h) {
                        highest_permissive = Some(usages);
                    }
                }
            }
            if min_reset.map_or(true, |r| pool.iterations_to_reset < r) {
                min_reset = Some(pool.iterations_to_reset);
            }
        }

        let usages = lowest_limit.or(highest_permissive).unwrap_or(0);
        let reset = min_reset.unwrap_or(0);
        (usages, reset)
    }
}
