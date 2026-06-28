/*
* SYSTEM ACTION
* Create a new organization
* Use config to determine details
* Remember that an org is just a variant of an actor
*/

use lawliet_types::action::CreateChannel;

use crate::{
    action::{
        Action, ActionInterface, ActionResponse, CreateAndGiveOrgAbility, CreateAndGivePassive,
    },
    actor::organization::{LeadershipStruct, OrgAbility},
    common::ActorKey,
};

use crate::action::ActionActor;
pub use crate::action::{CreateOrg, CreateOrgResponse};

impl ActionInterface for CreateOrg {
    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &ActionActor,
        version: crate::common::Version,
        mutate: bool,
    ) -> crate::action::ActionResult {
        actor.admin_or_system()?;

        let org_config = eng
            .config
            .org_config
            .get(&self.name)
            .expect("Organization unimplemented!");
        let abilities = org_config.abilities.clone();
        let passives = org_config.passives.clone();

        let channel_response = Action::CreateChannel(CreateChannel { loggable: true })
            .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::CreateChannel(data) = channel_response else {
            unreachable!();
        };
        let channel_id = data.id;

        let org_config = eng
            .config
            .org_config
            .get(&self.name)
            .expect("already validated org config");

        let id = if mutate {
            let mut leadership = None;
            if let Some(leadership_conf) = &org_config.leadership {
                let leadership_struct = LeadershipStruct {
                    leader: None,
                    transfer_policies: leadership_conf.transfer_policies,
                };
                leadership = Some(leadership_struct);
            }
            eng.world.add_org(self.name, leadership, channel_id)
        } else {
            ActorKey::default()
        };

        if mutate {
            for ability in abilities {
                let settings = OrgAbility {
                    require_roles: ability.require_roles.into_iter().collect(),
                    require_members: ability.require_members,
                    usage_policies: ability.usage_policies,
                };
                Action::CreateAndGiveOrgAbility(CreateAndGiveOrgAbility {
                    ability_name: ability.identifier.name,
                    variant: ability.identifier.variant,
                    org_id: id,
                    settings,
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }

            for passive in passives {
                Action::CreateAndGivePassive(CreateAndGivePassive {
                    actor_id: id,
                    passive_type: passive,
                    transferrable: false,
                    volatile: true,
                })
                .handle(eng, ctx, actor, version, mutate)?;
            }
        }

        Ok(ActionResponse::CreateOrg(CreateOrgResponse { id }))
    }
}
