use crate::{
    action::{
        actor::{add_state::AddState, player::give_role::GiveRole, remove_state::RemoveState},
        kidnapping::{
            create_kidnapping::CreateKidnapping,
            release_kidnapping::ReleaseKidnapping,
        },
        comms::{
            channel::{
                create_channel::CreateChannel, send_message::SendMessage, set_member::SetMember,
            },
            groupchat::{
                add_to_groupchat::AddToGroupchat, create_groupchat::CreateGroupchat,
                remove_from_groupchat::RemoveFromGroupchat, set_groupchat_owner::SetGroupchatOwner,
            },
            lounge::{
                create_lounge::CreateLounge, leave_lounge::LeaveLounge,
                remove_from_lounge::RemoveFromLounge,
            },
        },
        world::set_world_channel_override::SetWorldChannelOverride,
    },
    actor::{
        ActorDisplay,
        player::{OverrideSource, WorldChannelOverride},
        state::State,
    },
    channel::ChannelMember,
    config::world::WorldChannelName,
    lounge::LoungeVariant,
};

use crate::{
    Time,
    ability::AbilityBehaviour,
    action::{
        Action, ActionActor, ActionRequest, ActionResponse, ActionResult,
        ability::{
            add_link::AddLink, clear_links::ClearLinks,
            create_and_give_ability::CreateAndGiveAbility, use_ability::UseAbility,
        },
        actor::{
            org::{
                add_to_org::AddToOrg, change_org_leader::ChangeOrgLeader,
                create_and_give_org_ability::CreateAndGiveOrgAbility, create_org::CreateOrg,
                remove_from_org::RemoveFromOrg, set_leadership::SetLeadership,
                use_org_ability::UseOrgAbility,
            },
            player::{add_player::AddPlayer, kill::Kill, revive::Revive},
        },
        chargepool::add_charge_pool::AddChargePool,
        engine::null::Null,
        notebook::{
            create_and_give_notebook::CreateAndGiveNotebook, lend_notebook::LendNotebook,
            write_name::WriteName,
        },
        passive::create_and_give_passive::CreateAndGivePassive,
        poll::{add_vote::AddVote, create_poll::CreatePoll, remove_vote::RemoveVote},
        world::initialize_engine::InitializeEngine,
    },
    actor::organization::LeadershipTransferPolicies,
    chargepool::PoolLinkType,
    common::{
        AbilityKey, ActorKey, ChannelKey, ChargeCount, ChargePoolKey, GroupchatKey, KidnappingKey,
        LinkWeight, LoungeKey, NotebookKey, PassiveKey, PollKey,
    },
    kidnapping::{KidnappingSource, KidnappingType},
    config::{actor::organization::OrganizationName, role::Role},
    engine::{Engine, ExecutionResult},
    passive::PassiveType,
};

pub fn add_player(
    eng: &mut Engine,
    timestamp: Time,
    starting_role: Role,
    true_name: &str,
) -> ActorKey {
    let data = eng
        .execute(ActionRequest {
            timestamp,
            actor: ActionActor::System,
            payload: Action::AddPlayer(AddPlayer {
                true_name: String::from(true_name),
                starting_role,
            }),
        })
        .unwrap()
        .0;
    let ActionResponse::AddPlayer(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn quick_kill(
    eng: &mut Engine,
    timestamp: Time,
    allow_link_chaining: bool,
    sever_links: bool,
    set_books_dormant: bool,
    target: ActorKey,
) {
    eng.execute(ActionRequest {
        timestamp,
        actor: ActionActor::System,
        payload: Action::Kill(Kill {
            target_id: target,
            killer_id: None,
            death_message: None,
            silent: true,
            set_books_dormant,
            allow_link_chaining,
            sever_links,
        }),
    })
    .unwrap();
}

pub fn quick_revive(eng: &mut Engine, timestamp: Time, ignore_links: bool, target: ActorKey) {
    eng.execute(ActionRequest {
        timestamp,
        actor: ActionActor::System,
        payload: Action::Revive(Revive {
            target_id: target,
            ignore_links,
        }),
    })
    .unwrap();
}

pub fn quick_write(
    eng: &mut Engine,
    writer: ActorKey,
    timestamp: Time,
    notebook_id: NotebookKey,
    true_name: &str,
    delay: Time,
) -> ActionResult {
    let result = eng.execute(ActionRequest {
        actor: ActionActor::Player(writer),
        timestamp,
        payload: Action::WriteName(WriteName {
            true_name: true_name.into(),
            death_message: None,
            notebook_id,
            delay,
        }),
    });
    match result {
        Ok(response) => Ok(response.0),
        Err(err) => Err(err),
    }
}

pub fn null_action(eng: &mut Engine, time: Time) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::Null(Null {}),
    })
    .unwrap();
}

pub fn quick_lend(
    eng: &mut Engine,
    time: Time,
    notebook_id: NotebookKey,
    player_lending: ActorKey,
    lend_to: ActorKey,
) {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(player_lending),
        timestamp: time,
        payload: Action::LendNotebook(LendNotebook {
            notebook_id,
            target_id: lend_to,
        }),
    })
    .unwrap();
}

pub fn quick_notebook(eng: &mut Engine, time: Time, player: ActorKey, fake: bool) -> NotebookKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateAndGiveNotebook(CreateAndGiveNotebook {
                fake,
                actor_id: player,
                volatile: false,
            }),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateAndGiveNotebook(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn quick_passive(
    eng: &mut Engine,
    time: Time,
    player: ActorKey,
    passive_type: PassiveType,
    transferrable: bool,
) -> PassiveKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateAndGivePassive(CreateAndGivePassive {
                passive_type,
                transferrable,
                actor_id: player,
                volatile: false,
            }),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateAndGivePassive(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn create_poll(eng: &mut Engine, time: Time, action: CreatePoll) -> PollKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreatePoll(action),
        })
        .unwrap()
        .0;
    let ActionResponse::CreatePoll(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn add_vote(
    eng: &mut Engine,
    time: Time,
    poll_id: PollKey,
    voter_id: ActorKey,
    accept: bool,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(voter_id),
        timestamp: time,
        payload: Action::AddVote(AddVote { poll_id, accept }),
    })
}

pub fn remove_vote(
    eng: &mut Engine,
    time: Time,
    poll_id: PollKey,
    voter_id: ActorKey,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(voter_id),
        timestamp: time,
        payload: Action::RemoveVote(RemoveVote { poll_id }),
    })
}

pub fn default_kill(id: ActorKey) -> Action {
    Action::Kill(Kill {
        allow_link_chaining: true,
        death_message: None,
        killer_id: None,
        target_id: id,
        sever_links: true,
        silent: false,
        set_books_dormant: false,
    })
}

pub fn quick_ability(eng: &mut Engine, time: Time, args: CreateAndGiveAbility) -> AbilityKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateAndGiveAbility(args),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateAndGiveAbility(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn use_ability(
    eng: &mut Engine,
    time: Time,
    user_id: ActorKey,
    ability_id: AbilityKey,
    args: AbilityBehaviour,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(user_id),
        timestamp: time,
        payload: Action::UseAbility(UseAbility {
            ability_id,
            ability_args: args,
        }),
    })
}

pub fn quick_pool(eng: &mut Engine, time: Time, args: AddChargePool) -> ChargePoolKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::AddChargePool(args),
        })
        .unwrap()
        .0;
    let ActionResponse::AddChargePool(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn quick_link(
    eng: &mut Engine,
    time: Time,
    ability_id: AbilityKey,
    pool_id: ChargePoolKey,
    link_type: PoolLinkType,
    weight: LinkWeight,
) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::AddLink(AddLink {
            ability_id,
            pool_id,
            weight,
            link_type,
            volatile: false,
        }),
    })
    .unwrap();
}

pub fn quick_clear_links(eng: &mut Engine, time: Time, ability_id: AbilityKey) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::ClearLinks(ClearLinks { ability_id }),
    })
    .unwrap();
}

pub fn init_engine(eng: &mut Engine) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: 0,
        payload: Action::InitializeEngine(InitializeEngine { seed: 0 }),
    })
    .unwrap();
}

pub fn add_org(eng: &mut Engine, time: Time, org: OrganizationName) -> ActorKey {
    let data = eng
        .execute(ActionRequest {
            timestamp: time,
            actor: ActionActor::System,
            payload: Action::CreateOrg(CreateOrg { name: org }),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateOrg(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn add_to_org(
    eng: &mut Engine,
    time: Time,
    org: ActorKey,
    actor: ActorKey,
    leader: bool,
    og: bool,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::AddToOrg(AddToOrg {
            actor_id: actor,
            leader,
            og,
            org_id: org,
        }),
    })
}

pub fn remove_from_org(
    eng: &mut Engine,
    time: Time,
    org: ActorKey,
    actor: ActorKey,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::RemoveFromOrg(RemoveFromOrg {
            actor_id: actor,
            org_id: org,
        }),
    })
}

pub fn set_leadership(
    eng: &mut Engine,
    time: Time,
    org: ActorKey,
    policies: Option<LeadershipTransferPolicies>,
) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::SetLeadership(SetLeadership {
            policies,
            org_id: org,
        }),
    })
    .unwrap();
}

pub fn change_leader(
    eng: &mut Engine,
    time: Time,
    org: ActorKey,
    actor: Option<ActorKey>,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::ChangeOrgLeader(ChangeOrgLeader {
            org_id: org,
            new_leader: actor,
        }),
    })
}

pub fn quick_org_ability(
    eng: &mut Engine,
    time: Time,
    args: CreateAndGiveOrgAbility,
) -> AbilityKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateAndGiveOrgAbility(args),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateAndGiveOrgAbility(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn use_org_ability(
    eng: &mut Engine,
    time: Time,
    user_id: ActorKey,
    org_id: ActorKey,
    ability_id: AbilityKey,
    args: AbilityBehaviour,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(user_id),
        timestamp: time,
        payload: Action::UseOrgAbility(UseOrgAbility {
            ability_id,
            ability_args: args,
            org_id,
        }),
    })
}

pub fn force_charges(eng: &mut Engine, time: Time, ability_id: AbilityKey, charges: ChargeCount) {
    quick_clear_links(eng, 0, ability_id);
    quick_pool(
        eng,
        0,
        AddChargePool {
            base_charges: charges,
            base_reset_time: 1,
        },
    );
}

pub fn add_state(eng: &mut Engine, time: Time, actor_id: ActorKey, state: State) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::AddState(AddState { actor_id, state }),
    })
    .unwrap();
}

pub fn remove_state(eng: &mut Engine, time: Time, actor_id: ActorKey, state: State) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::RemoveState(RemoveState { actor_id, state }),
    })
    .unwrap();
}

pub fn create_channel(eng: &mut Engine, time: Time, loggable: bool) -> ChannelKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateChannel(CreateChannel { loggable }),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateChannel(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn set_member(
    eng: &mut Engine,
    time: Time,
    player_id: ActorKey,
    channel_id: ChannelKey,
    settings: Option<ChannelMember>,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::SetMember(SetMember {
            player_id,
            channel_id,
            settings,
        }),
    })
}

pub fn send_message(
    eng: &mut Engine,
    time: Time,
    player_id: ActorKey,
    channel_id: ChannelKey,
    display: ActorDisplay,
    content: &str,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(player_id),
        timestamp: time,
        payload: Action::SendMessage(SendMessage {
            channel_id,
            display,
            content: content.into(),
        }),
    })
}

pub fn create_gc(eng: &mut Engine, time: Time) -> GroupchatKey {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateGroupchat(CreateGroupchat {}),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateGroupchat(response) = data else {
        unreachable!()
    };
    response.id
}

pub fn add_to_gc(
    eng: &mut Engine,
    time: Time,
    actor: ActionActor,
    gc_id: GroupchatKey,
    player_id: ActorKey,
    owner: bool,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor,
        timestamp: time,
        payload: Action::AddToGroupchat(AddToGroupchat {
            groupchat_id: gc_id,
            player_id,
            owner,
        }),
    })
}

pub fn remove_from_gc(
    eng: &mut Engine,
    time: Time,
    actor: ActionActor,
    gc_id: GroupchatKey,
    player_id: ActorKey,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor,
        timestamp: time,
        payload: Action::RemoveFromGroupchat(RemoveFromGroupchat {
            groupchat_id: gc_id,
            player_id,
        }),
    })
}

pub fn set_gc_owner(
    eng: &mut Engine,
    time: Time,
    actor: ActionActor,
    gc_id: GroupchatKey,
    owner: Option<ActorKey>,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor,
        timestamp: time,
        payload: Action::SetGroupchatOwner(SetGroupchatOwner {
            groupchat_id: gc_id,
            owner,
        }),
    })
}

pub fn create_lounge(
    eng: &mut Engine,
    time: Time,
    variant: LoungeVariant,
) -> (LoungeKey, ChannelKey) {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateLounge(CreateLounge { variant }),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateLounge(response) = data else {
        unreachable!()
    };
    (response.lounge_id, response.channel_id)
}

pub fn leave_lounge(
    eng: &mut Engine,
    time: Time,
    player_id: ActorKey,
    lounge_id: LoungeKey,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::Player(player_id),
        timestamp: time,
        payload: Action::LeaveLounge(LeaveLounge { lounge_id }),
    })
}

pub fn remove_from_lounge(
    eng: &mut Engine,
    time: Time,
    player_id: ActorKey,
    lounge_id: LoungeKey,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::RemoveFromLounge(RemoveFromLounge {
            lounge_id,
            player_id,
        }),
    })
}

pub fn give_role(eng: &mut Engine, time: Time, target_id: ActorKey, role: Role) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::GiveRole(GiveRole { target_id, role }),
    })
    .unwrap();
}

pub fn set_world_channel_override(
    eng: &mut Engine,
    time: Time,
    player_id: ActorKey,
    channel_name: WorldChannelName,
    source: OverrideSource,
    priority: u8,
    override_data: Option<WorldChannelOverride>,
) -> ExecutionResult {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::SetWorldChannelOverride(SetWorldChannelOverride {
            player_id,
            channel_name,
            source,
            priority,
            override_data,
        }),
    })
}

pub fn create_kidnapping(
    eng: &mut Engine,
    time: Time,
    victim_id: ActorKey,
    kidnapping_type: KidnappingType,
    source: KidnappingSource,
) -> (KidnappingKey, ChannelKey) {
    let data = eng
        .execute(ActionRequest {
            actor: ActionActor::System,
            timestamp: time,
            payload: Action::CreateKidnapping(CreateKidnapping {
                victim_id,
                kidnapping_type,
                source,
            }),
        })
        .unwrap()
        .0;
    let ActionResponse::CreateKidnapping(r) = data else {
        unreachable!()
    };
    let ch = eng.world.get_kidnapping(r.id).unwrap().channel_id;
    (r.id, ch)
}

pub fn release_kidnapping(eng: &mut Engine, time: Time, kidnapping_id: KidnappingKey) {
    eng.execute(ActionRequest {
        actor: ActionActor::System,
        timestamp: time,
        payload: Action::ReleaseKidnapping(ReleaseKidnapping { kidnapping_id, forced: false }),
    })
    .unwrap();
}
