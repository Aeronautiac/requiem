use crate::{
    common::{AbilityKey, ActorKey, MemberCount},
    config::actor::organization::OrganizationName,
};
use indexmap::{IndexMap, IndexSet};

pub use lawliet_types::organization::{
    OrgAbility, OrgAbilityPolicy, OrgAbilityPolicies,
    LeadershipTransferPolicy, LeadershipTransferPolicies,
};

// TODO:
// Org behaviours:
// - Organizations shall have a set of actives and passives.
// - When a player joins an org, a passive link is created between the player and the org (the
// player effectively has the org's passives).
// - Orgs have a set of active abilities. Each entry in this set contains metadata about the usage
// mode. For example, an org's active may be used without a vote or may be used only by the current
// leader.
// - When players leave an org, their links to that org shall be severed.
// - Orgs have an "invite" charge pool which many abilities draw from.
//
// Some examples:
// - Kira's Kingdom's invite ability requires a vote. They do not have a leader, so it is not leader
// only.
// - Task Force's silent prosecution ability does not require a vote and can be used by any member.
// - If Kira's Kingdom uses blackout, they are given the "Wanted" passive, and any members who were
// in Kira's Kingdom when blackout occurred are also given it explicitly. Any members who joined
// after will still have the passive through the passive link, though if they leave they are not
// permanently stained.
// - Task Force's invite and outsource abilities can only be used by the chief and do not require a
// vote.

// need a new action for using org abilities (create a vote or instant use, check permissions and
// org state)

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct LeadershipStruct {
    pub leader: Option<ActorKey>,
    pub transfer_policies: LeadershipTransferPolicies,
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct OrgMember {
    pub og: bool,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Organization {
    pub leadership_struct: Option<LeadershipStruct>,
    pub members: IndexMap<ActorKey, OrgMember>,
    pub blacklist: IndexSet<ActorKey>,
    pub abilities: IndexMap<AbilityKey, OrgAbility>,
    pub org_name: OrganizationName,
}

impl Organization {
    pub fn new(name: OrganizationName, leadership_struct: Option<LeadershipStruct>) -> Self {
        Organization {
            leadership_struct,
            members: IndexMap::new(),
            abilities: IndexMap::new(),
            blacklist: IndexSet::new(),
            org_name: name,
        }
    }

    pub fn has_member(&self, id: ActorKey) -> bool {
        self.members.contains_key(&id)
    }

    pub fn is_blacklisted(&self, id: ActorKey) -> bool {
        self.blacklist.contains(&id)
    }

    /// this will replace the old leader (if applicable)
    pub fn add_member(&mut self, id: ActorKey, og: bool) {
        self.members.insert(id, OrgMember { og });
    }

    /// if this member was the leader, there will be no leader after this
    pub fn remove_member(&mut self, id: ActorKey) {
        self.members.swap_remove(&id);
        if let Some(leadership_struct) = &mut self.leadership_struct
            && leadership_struct.leader == Some(id)
        {
            leadership_struct.leader = None;
        }
    }

    /// count number of members matching a certain condition
    pub fn member_count(&self, condition: impl Fn(ActorKey, &OrgMember) -> bool) -> MemberCount {
        let mut count = 0;
        for (id, member) in self.members.iter() {
            if condition(*id, member) {
                count += 1;
            }
        }
        count
    }

    pub fn get_leader(&self) -> Option<ActorKey> {
        if let Some(leadership) = &self.leadership_struct {
            leadership.leader
        } else {
            None
        }
    }

    pub fn add_ability(&mut self, id: AbilityKey, settings: OrgAbility) {
        self.abilities.insert(id, settings);
    }
}
