// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, mem, sync::Arc};

use consensus_config::AuthorityIndex;
use itertools::Itertools;
use parking_lot::RwLock;
use tracing::info;

use crate::{
    base_committer::BaseCommitter,
    block::{Round, Slot, GENESIS_ROUND},
    commit::{CommitAPI, DecidedLeader, Decision, TrustedCommit},
    context::Context,
    dag_state::DagState,
};

#[cfg(test)]
#[path = "tests/universal_committer_tests.rs"]
mod universal_committer_tests;

#[cfg(test)]
#[path = "tests/pipelined_committer_tests.rs"]
mod pipelined_committer_tests;

/// A universal committer uses a collection of committers to commit a sequence of leaders.
/// It can be configured to use a combination of different commit strategies, including
/// multi-leaders, backup leaders, and pipelines.
pub(crate) struct UniversalCommitter {
    /// The per-epoch configuration of this authority.
    context: Arc<Context>,
    /// In memory block store representing the dag state
    dag_state: Arc<RwLock<DagState>>,
    /// The list of committers for multi-leader or pipelining
    committers: Vec<BaseCommitter>,
}

impl UniversalCommitter {
    /// Try to decide part of the dag. This function is idempotent and returns an ordered list of
    /// decided leaders.
    #[tracing::instrument(skip_all, fields(last_decided = %last_decided))]
    pub(crate) fn try_decide(&self, last_decided: Slot) -> Vec<DecidedLeader> {
        let highest_accepted_round = self.dag_state.read().highest_accepted_round();

        // Try to decide as many leaders as possible, starting with the highest round.
        let mut leaders = VecDeque::new();

        let last_round = match self
            .context
            .protocol_config
            .mysticeti_num_leaders_per_round()
        {
            Some(1) => {
                // Ensure that we don't commit any leaders from the same round as last_decided
                // until we have full support for multi-leader per round.
                // This can happen when we are on a leader schedule boundary and the leader
                // elected for the round changes with the new schedule.
                last_decided.round + 1
            }
            _ => last_decided.round,
        };

        // try to commit a leader up to the highest_accepted_round - 2. There is no
        // reason to try and iterate on higher rounds as in order to make a direct
        // decision for a leader at round R we need blocks from round R+2 to figure
        // out that enough certificates and support exist to commit a leader.
        'outer: for round in (last_round..=highest_accepted_round.saturating_sub(2)).rev() {
            for committer in self.committers.iter().rev() {
                // Skip committers that don't have a leader for this round.
                let Some(slot) = committer.elect_leader(round) else {
                    tracing::debug!("No leader for round {round}, skipping");
                    continue;
                };

                // now that we reached the last committed leader we can stop the commit rule
                if slot == last_decided {
                    tracing::debug!("Reached last committed {slot}, now exit");
                    break 'outer;
                }

                tracing::debug!("Trying to decide {slot} with {committer}",);

                // Try to directly decide the leader.
                let mut status = committer.try_direct_decide(slot);
                tracing::debug!("Outcome of direct rule: {status}");

                // If we can't directly decide the leader, try to indirectly decide it.
                if status.is_decided() {
                    leaders.push_front((status, Decision::Direct));
                } else {
                    status = committer.try_indirect_decide(slot, leaders.iter().map(|(x, _)| x));
                    tracing::debug!("Outcome of indirect rule: {status}");
                    leaders.push_front((status, Decision::Indirect));
                }
            }
        }

        // The decided sequence is the longest prefix of decided leaders.
        let mut decided_leaders = Vec::new();
        for (leader, decision) in leaders {
            if leader.round() == GENESIS_ROUND {
                continue;
            }
            let Some(decided_leader) = leader.into_decided_leader() else {
                break;
            };
            self.update_metrics(&decided_leader, decision);
            decided_leaders.push(decided_leader);
        }
        tracing::debug!("Decided {decided_leaders:?}");
        decided_leaders
    }

    // Try to decide which of the synced commits will have to be committed next respecting the `commits_until_update` limit. If provided `commits_until_update` is zero, it will panic.
    // The function returns the list of decided leaders and updates in place the remaining synced commits. If empty vector is returned, it means that
    // there are no synced commits to be committed as `synced_commits` is either empty or all of the synced commits are already committed.
    #[tracing::instrument(skip_all)]
    pub(crate) fn try_decide_synced(
        &mut self,
        synced_commits: &mut Vec<TrustedCommit>,
        commits_until_update: usize,
    ) -> Vec<DecidedLeader> {
        // If GC is disabled then should not run any of this logic.
        if !self.dag_state.read().gc_enabled() {
            return Vec::new();
        }

        assert!(
            commits_until_update > 0,
            "commits_until_update should be greater than 0"
        );

        let last_commit_index = self.dag_state.read().last_commit_index();

        // If there are synced committed leaders, check that the first synced committed leader which is higher than the last decided one has not gaps.
        while !synced_commits.is_empty() {
            let synced_commit = synced_commits
                .first()
                .expect("Synced commits should not be empty");
            if synced_commit.index() <= last_commit_index {
                info!(
                    "Skip commit for index {} as it is already committed with last commit index {}",
                    synced_commit.index(),
                    last_commit_index
                );
                synced_commits.remove(0);
            } else {
                // Make sure that the first we do find is the next one in line and there is no gap.
                if synced_commit.index() != last_commit_index + 1 {
                    panic!("Gap found between the synced commits and the last committed index. Expected next commit index to be {}, but found {}", last_commit_index + 1, synced_commit.index());
                }

                // now break as we want to process the rest of the committed leaders
                break;
            }
        }

        if synced_commits.is_empty() {
            return Vec::new();
        }

        // We keep only the number of leaders that can be committed before the next leader schedule change.
        let to_commit = if synced_commits.len() >= commits_until_update {
            // Now keep only the leaders that can be committed before the next leader schedule change, and just leave the rest so we can process them in the next iteration.
            synced_commits
                .drain(..commits_until_update)
                .collect::<Vec<_>>()
        } else {
            // Otherwise just take all of them and leave the `synced_commits` empty.
            mem::take(synced_commits)
        };

        let dag_state = self.dag_state.read();
        let sequenced_leaders = to_commit
            .iter()
            .map(|commit| {
                let leader = DecidedLeader::Commit(dag_state.get_block(&commit.leader()).unwrap());
                self.update_metrics(&leader, Decision::Synced);
                leader
            })
            .collect::<Vec<_>>();

        tracing::info!(
            "Decided {} synced leaders: {}",
            sequenced_leaders.len(),
            to_commit.iter().map(|c| c.leader().to_string()).join(",")
        );

        sequenced_leaders
    }

    /// Return list of leaders for the round.
    /// Can return empty vec if round does not have a designated leader.
    pub(crate) fn get_leaders(&self, round: Round) -> Vec<AuthorityIndex> {
        self.committers
            .iter()
            .filter_map(|committer| committer.elect_leader(round))
            .map(|l| l.authority)
            .collect()
    }

    /// Update metrics.
    fn update_metrics(&self, decided_leader: &DecidedLeader, decision: Decision) {
        let decision_str = match decision {
            Decision::Direct => "direct",
            Decision::Indirect => "indirect",
            Decision::Synced => "synced",
        };
        let status = match decided_leader {
            DecidedLeader::Commit(..) => format!("{decision_str}-commit"),
            DecidedLeader::Skip(..) => format!("{decision_str}-skip"),
        };
        let leader_host = &self
            .context
            .committee
            .authority(decided_leader.slot().authority)
            .hostname;
        self.context
            .metrics
            .node_metrics
            .committed_leaders_total
            .with_label_values(&[leader_host, &status])
            .inc();
    }
}

/// A builder for a universal committer. By default, the builder creates a single
/// base committer, that is, a single leader and no pipeline.
pub(crate) mod universal_committer_builder {
    use super::*;
    use crate::{
        base_committer::BaseCommitterOptions, commit::DEFAULT_WAVE_LENGTH,
        leader_schedule::LeaderSchedule,
    };

    pub(crate) struct UniversalCommitterBuilder {
        context: Arc<Context>,
        leader_schedule: Arc<LeaderSchedule>,
        dag_state: Arc<RwLock<DagState>>,
        wave_length: Round,
        number_of_leaders: usize,
        pipeline: bool,
    }

    impl UniversalCommitterBuilder {
        pub(crate) fn new(
            context: Arc<Context>,
            leader_schedule: Arc<LeaderSchedule>,
            dag_state: Arc<RwLock<DagState>>,
        ) -> Self {
            Self {
                context,
                leader_schedule,
                dag_state,
                wave_length: DEFAULT_WAVE_LENGTH,
                number_of_leaders: 1,
                pipeline: false,
            }
        }

        #[allow(unused)]
        pub(crate) fn with_wave_length(mut self, wave_length: Round) -> Self {
            self.wave_length = wave_length;
            self
        }

        pub(crate) fn with_number_of_leaders(mut self, number_of_leaders: usize) -> Self {
            self.number_of_leaders = number_of_leaders;
            self
        }

        pub(crate) fn with_pipeline(mut self, pipeline: bool) -> Self {
            self.pipeline = pipeline;
            self
        }

        pub(crate) fn build(self) -> UniversalCommitter {
            let mut committers = Vec::new();
            let pipeline_stages = if self.pipeline { self.wave_length } else { 1 };
            for round_offset in 0..pipeline_stages {
                for leader_offset in 0..self.number_of_leaders {
                    let options = BaseCommitterOptions {
                        wave_length: self.wave_length,
                        round_offset,
                        leader_offset: leader_offset as Round,
                    };
                    let committer = BaseCommitter::new(
                        self.context.clone(),
                        self.leader_schedule.clone(),
                        self.dag_state.clone(),
                        options,
                    );
                    committers.push(committer);
                }
            }

            UniversalCommitter {
                context: self.context,
                dag_state: self.dag_state,
                committers,
            }
        }
    }
}
