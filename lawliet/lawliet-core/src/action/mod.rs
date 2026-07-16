pub use lawliet_types::action::*;

use crate::{common::Version, engine::Engine};

pub mod ability;
pub mod actor;
pub mod chargepool;
pub mod comms;
pub mod engine;
pub mod incarceration;
pub mod kidnapping;
pub mod notebook;
pub mod passive;
pub mod poll;
pub mod prosecution;
pub mod update;
pub mod world;

pub type ActionResult = Result<ActionResponse, ActionError>;

pub trait ActionInterface {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult;
}

// this is yucky, but it results in significant performance gains
impl ActionInterface for Action {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        match self {
            Action::ChangeOrgLeader(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ResignLeadership(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::Kill(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddState(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::Revive(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddPlayer(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddNotebook(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::GiveNotebook(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::WriteName(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::LendNotebook(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ScheduleKill(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::RemoveState(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::GiveRole(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::DestroyAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UseAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ScheduleRevive(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::GiveAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddPassive(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::DestroyPassive(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::GivePassive(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SeverLinks(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateActorLinks(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::PurgeVolatiles(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateAndGiveAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateAndGiveNotebook(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::DestroyNotebook(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateAndGivePassive(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::TakeNotebook(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetNotebookPossession(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::Null(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::Crash(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetBorrowersToOwners(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetBooksDormant(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ReturnDormantBooks(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::NotebookScheduledKill(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::TryDeleteChargePool(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::InitializeWorld(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddChargePool(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ClearVolatileLinks(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UseOrgAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::Update(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdatePolls(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreatePoll(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::PollTimeout(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ScheduleJob(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddVote(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::RemoveVote(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::PollCleanup(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddToOrg(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::RemoveFromOrg(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateOrg(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SystemUseOrgAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddCharges(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddLink(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::RemoveLink(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ClearLinks(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateOrgs(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetLeadership(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::GiveOrgAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateAndGiveOrgAbility(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SendMessage(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateChannel(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::DestroyChannel(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetMember(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetLoggable(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetTrueName(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateLounge(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdateContactChannels(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::LeaveLounge(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::RemoveFromLounge(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddToGroupchat(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateGroupchat(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetGroupchatOwner(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::RemoveFromGroupchat(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateBug(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ArchiveBug(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::DestroyBug(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::StartProsecution(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetCustody(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AdvanceProsecution(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SignalReady(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SelectLawyer(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CullProsecutions(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::TerminateProsecution(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdateProsecutionChannels(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdateProsecutions(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::AddToWorldChannels(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdateWorldChannelPerms(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetWorldChannelOverride(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::InitializeEngine(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::SetRandomSeed(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::DeferredCmds(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdateBugVisibilities(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ProsecutionVoteRes(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateKidnapping(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ReleaseKidnapping(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CullKidnappings(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdateKidnapChannels(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::UpdatePrisonChannel(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreateIncarceration(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::ReleaseIncarceration(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::TimedIncarceration(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CullIncarcerations(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::CreatePersonalChannel(a) => a.handle(eng, ctx, actor, version, mutate),
            Action::NextIteration(a) => a.handle(eng, ctx, actor, version, mutate),
        }
    }
}

pub trait ActionExt {
    fn execute(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
    ) -> ActionResult;

    fn validate(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
    ) -> ActionResult;
}

impl ActionExt for Action {
    fn execute(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
    ) -> ActionResult {
        // Keep ctx.mutate in lockstep with the pass so command pushes are enabled here and
        // suppressed inside any nested validate probe (saved/restored so nesting is safe).
        let prev = ctx.mutate;
        ctx.mutate = true;
        let result = self.handle(eng, ctx, actor, version, true);
        Action::Update(Update {})
            .handle(eng, ctx, &ActionActor::System, version, true)
            .expect("Update action has failed");
        ctx.mutate = prev;
        result
    }

    fn validate(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
    ) -> ActionResult {
        // A dry pass: suppress command pushes (restored after, so a probe run mid-execute
        // doesn't leak the probed action's commands into the real stream).
        let prev = ctx.mutate;
        ctx.mutate = false;
        let result = self.handle(eng, ctx, actor, version, false);
        ctx.mutate = prev;
        result
    }
}
