use smallvec::smallvec;
use std::{cmp::Ordering, collections::BinaryHeap};

use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::{Time, action::ActionRequest, common::JobID};

#[derive(PartialEq, Eq, Debug)]
pub struct QueueEntry {
    pub id: JobID,
    pub time: Time,
}

#[derive(Debug)]
pub struct Job {
    pub cancelled: bool,
    pub request: ActionRequest,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .time
            .cmp(&self.time)
            .then_with(|| other.id.cmp(&self.id))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct Jobs {
    jobs: IndexMap<JobID, Job>,
    job_queue: BinaryHeap<QueueEntry>,
    next_job_id: JobID,
    cancelled_count: usize,
}

impl Jobs {
    pub fn new() -> Self {
        Jobs {
            jobs: IndexMap::new(),
            job_queue: BinaryHeap::new(),
            cancelled_count: 0,
            next_job_id: 0,
        }
    }

    pub fn cancel_all_cond<F>(&mut self, cond: F, mutate: bool) -> usize
    where
        F: Fn(&Job) -> bool,
    {
        let mut to_cancel: SmallVec<[JobID; 8]> = smallvec![];
        for (id, job) in self.jobs.iter_mut() {
            if !job.cancelled && cond(job) {
                to_cancel.push(*id);
            }
        }
        let c = to_cancel.len();
        if mutate {
            for id in to_cancel {
                self.cancel_id(id);
            }
        }
        c
    }

    // only remove from the map and update count
    // do not call without removing from the heap
    fn remove_job(&mut self, id: JobID) -> Option<Job> {
        let job = self.jobs.swap_remove(&id);
        if let Some(job_data) = &job
            && job_data.cancelled
        {
            self.cancelled_count -= 1;
        }
        job
    }

    pub fn cancel_id(&mut self, id: JobID) -> bool {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.cancelled = true;
            self.cancelled_count += 1;
            self.pop_cancelled();
            true
        } else {
            false
        }
    }

    fn pop_cancelled(&mut self) {
        while !self.jobs.is_empty() {
            let id = self.job_queue.peek().unwrap().id;
            let job = self.jobs.get_mut(&id).unwrap();
            if job.cancelled {
                self.remove_job(id);
                self.pop();
            } else {
                break;
            }
        }
    }

    pub fn peek(&self) -> Option<&Job> {
        let entry = self.job_queue.peek()?;
        let id = entry.id;
        self.view(id)
    }

    pub fn push(&mut self, request: ActionRequest) -> JobID {
        let id = self.next_job_id;
        let time = request.timestamp;
        self.jobs.insert(
            id,
            Job {
                cancelled: false,
                request,
            },
        );
        self.job_queue.push(QueueEntry { time, id });
        self.next_job_id += 1;
        id
    }

    pub fn pop(&mut self) -> Option<Job> {
        self.pop_cancelled();
        let entry_result = self.job_queue.pop();
        let mut job = None;
        if let Some(entry) = entry_result {
            job = self.remove_job(entry.id);
        }
        self.pop_cancelled();
        job
    }

    pub fn is_empty(&self) -> bool {
        self.job_queue.is_empty() || self.cancelled_count == self.job_queue.len()
    }

    pub fn view(&self, id: JobID) -> Option<&Job> {
        self.jobs.get(&id)
    }
}
