pub use lawliet_types::ability::{AbilityBehaviour, AbilityName};

use crate::{
    action::{ActionActor, ActionContext, ActionError},
    chargepool::{ChargeCondition, ChargeConditions, PoolLink, PoolLinkType},
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
pub mod background_check;
pub mod blackout;
pub mod bug;
pub mod civilian_arrest;
pub mod contact;
pub mod create_groupchat;
pub mod fake_lounge;
pub mod false_anonymous_contact;
pub mod force_invite;
pub mod gun;
pub mod ipp;
pub mod leader_resign;
pub mod notebook_reveal;
pub mod outsource;
pub mod prosecute;
pub mod pseudocide;
pub mod public_kidnap;
pub mod shinigami_sacrifice;
pub mod tap_in;
pub mod true_name_invite;
pub mod true_name_reroll;
pub mod true_name_reveal;
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
            AbilityBehaviour::Bug(a) => a.ability_name(),
            AbilityBehaviour::CreateGroupchat(a) => a.ability_name(),
            AbilityBehaviour::FabricateLounge(a) => a.ability_name(),
            AbilityBehaviour::FalseAnonymousContact(a) => a.ability_name(),
            AbilityBehaviour::Ipp(a) => a.ability_name(),
            AbilityBehaviour::Prosecute(a) => a.ability_name(),
            AbilityBehaviour::TrueNameInvite(a) => a.ability_name(),
            AbilityBehaviour::ForceInvite(a) => a.ability_name(),
            AbilityBehaviour::BackgroundCheck(a) => a.ability_name(),
            AbilityBehaviour::Outsource(a) => a.ability_name(),
            AbilityBehaviour::LeaderResign(a) => a.ability_name(),
            AbilityBehaviour::TrueNameReveal(a) => a.ability_name(),
            AbilityBehaviour::NotebookReveal(a) => a.ability_name(),
            AbilityBehaviour::CivilianArrest(a) => a.ability_name(),
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
            AbilityBehaviour::Bug(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::CreateGroupchat(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::FabricateLounge(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::FalseAnonymousContact(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::Ipp(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::Prosecute(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::TrueNameInvite(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::ForceInvite(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::BackgroundCheck(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::Outsource(a) => a.handle(eng, ctx, actor, ability, version, mutate),
            AbilityBehaviour::LeaderResign(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::TrueNameReveal(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::NotebookReveal(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
            AbilityBehaviour::CivilianArrest(a) => {
                a.handle(eng, ctx, actor, ability, version, mutate)
            }
        }
    }
}

// a volatile link is destroyed when the owner changes
// (no Ord: BitFlags isn't ordered, and these only ever live in a hash-based IndexSet)
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct AbilityPoolLink {
    pub volatile: bool,
    // when this link's pool is actually subtracted, based on the ability's returned
    // status (checking to gate usage happens regardless of this)
    pub condition: ChargeConditions,
    pub link: PoolLink,
}

// The outcome an ability reports back so use_ability can decide which linked pools to
// subtract. Every pool is still checked up front to gate usage; only subtraction is
// conditional.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AbilityStatus {
    Success,
    Failure,
}

pub type AbilityResult = Result<AbilityStatus, ActionError>;

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
        condition: ChargeConditions,
    ) -> Option<AbilityPoolLink> {
        let removed: Option<AbilityPoolLink> = self
            .pool_links
            .extract_if(.., |l| l.link.link_dest == link_dest)
            .next();
        self.pool_links.insert(AbilityPoolLink {
            volatile,
            condition,
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
                PoolLinkType::Restrictive => {
                    if let Some(lowest) = lowest_limit
                        && limit < lowest
                    {
                        lowest_limit = Some(limit);
                    } else if lowest_limit.is_none() {
                        lowest_limit = Some(limit);
                    }
                }
                PoolLinkType::Permissive => {
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

    // Usage counts split by outcome. With conditional charge subtraction, the number of
    // successful uses left can differ from the number of failed uses left (e.g. an
    // OnSuccess-only Invite pool limits only successes, while a both-outcomes attempts
    // pool limits both). Returns (success_usages, failure_usages, iterations_to_reset);
    // reset is the soonest reset across all pools, independent of the usage constraint.
    pub fn get_ability_view_counts(
        &self,
        eng: &Engine,
    ) -> (ChargeCount, ChargeCount, crate::common::IterationCount) {
        let mut min_reset: Option<crate::common::IterationCount> = None;
        for link_container in self.pool_links.iter() {
            let pool = eng
                .world
                .get_charge_pool(link_container.link.link_dest)
                .expect("expected valid link destination");
            if min_reset.map_or(true, |r| pool.iterations_to_reset < r) {
                min_reset = Some(pool.iterations_to_reset);
            }
        }

        let success = self.outcome_usages(eng, ChargeCondition::OnSuccess);
        let failure = self.outcome_usages(eng, ChargeCondition::OnFailure);
        (success, failure, min_reset.unwrap_or(0))
    }

    // How many times the ability can be used with a given outcome, based on which pools
    // that outcome actually subtracts: the tightest restrictive pool it consumes (or the
    // widest permissive one). If the outcome consumes no limiting pool, fall back to the
    // overall gate (min over all restrictive pools) so the number stays finite. Zero if
    // the ability can't currently be used at all (some restrictive pool is empty).
    fn outcome_usages(&self, eng: &Engine, outcome: ChargeCondition) -> ChargeCount {
        let mut lowest_consumed: Option<ChargeCount> = None;
        let mut highest_permissive: Option<ChargeCount> = None;
        let mut gate_limit: Option<ChargeCount> = None;
        let mut usable = true;

        for link_container in self.pool_links.iter() {
            let link = &link_container.link;
            let pool = eng
                .world
                .get_charge_pool(link.link_dest)
                .expect("expected valid link destination");
            let usages = pool.charges / link.weight;
            let consumed = link_container.condition.contains(outcome);
            match link.link_type {
                PoolLinkType::Restrictive => {
                    if !pool.can_use(link) {
                        usable = false;
                    }
                    if gate_limit.map_or(true, |g| usages < g) {
                        gate_limit = Some(usages);
                    }
                    if consumed && lowest_consumed.map_or(true, |l| usages < l) {
                        lowest_consumed = Some(usages);
                    }
                }
                PoolLinkType::Permissive => {
                    if consumed && highest_permissive.map_or(true, |h| usages > h) {
                        highest_permissive = Some(usages);
                    }
                }
            }
        }

        if !usable {
            return 0;
        }
        lowest_consumed
            .or(highest_permissive)
            .or(gate_limit)
            .unwrap_or(0)
    }
}
