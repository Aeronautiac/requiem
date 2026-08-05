/*
* SYSTEM ACTION
* Change the leader of an org
* The new leader can either be None or Some(leader_id)
* The new leader must already be in the org
*/

// notify existing leaders that leadership has changed

use lawliet_types::command::{Command, CommandRecipient};

use crate::{
    action::{ActionError, ActionInterface, ActionResponse},
    helpers::{get_actor, get_org, get_org_mut},
};

use crate::action::ActionActor;
pub use crate::action::{ChangeOrgLeader, ChangeOrgLeaderResponse};

impl ActionInterface for ChangeOrgLeader {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        _version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let time = eng.time;
        let org = get_org(eng, self.org_id)?;
        if let Some(new_leader) = self.new_leader {
            if !org.has_member(new_leader) {
                return Err(ActionError::PlayerNotInOrg);
            }
            get_actor(eng, new_leader)?;
        }

        let org = get_org_mut(eng, self.org_id)?;
        if let Some(leadership_struct) = &mut org.leadership_struct {
            if self.new_leader == leadership_struct.leader {
                return Err(ActionError::AlreadyLeader);
            }

            // tell old leader theyre no longer the leader
            if let Some(leader) = &leadership_struct.leader {
                ctx.push_cmd(
                    Command::LeaderStatus {
                        org_id: self.org_id,
                        leader: false,
                    },
                    CommandRecipient::Actor(*leader),
                    time,
                );
            }

            if mutate {
                leadership_struct.leader = self.new_leader;
            }

            // tell new leader theyre now the leader
            if let Some(new_leader) = self.new_leader {
                ctx.push_cmd(
                    Command::LeaderStatus {
                        org_id: self.org_id,
                        leader: true,
                    },
                    CommandRecipient::Actor(new_leader),
                    time,
                );
            }

            // tell the admin what the new leader is
            ctx.push_cmd(
                Command::OrgLeader {
                    leader: self.new_leader,
                    org_id: self.org_id,
                },
                CommandRecipient::System,
                time,
            );
        } else {
            return Err(ActionError::OrgDoesntHaveLeadership);
        }

        Ok(ActionResponse::ChangeOrgLeader(ChangeOrgLeaderResponse {}))
    }
}
