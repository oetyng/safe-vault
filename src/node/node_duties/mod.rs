// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

mod elder_constellation;
mod genesis;
pub mod messaging;
mod msg_analysis;
mod network_events;

use self::{
    elder_constellation::ElderConstellation,
    genesis::{GenesisAccumulation, GenesisProposal},
};
use crate::{
    network_state::{AdultReader, ElderDynamics, NodeSigning},
    node::{
        adult_duties::AdultDuties,
        elder_duties::ElderDuties,
        node_duties::messaging::Messaging,
        node_ops::{
            ElderDuty, NetworkDuties, NetworkDuty, NodeDuty, NodeMessagingDuty, OutgoingMsg,
            RewardCmd, RewardDuty,
        },
        NodeInfo,
    },
    AdultState, ElderState, Error, Network, NodeState, Result,
};
use log::{debug, info, trace};
use msg_analysis::ReceivedMsgAnalysis;
use network_events::NetworkEvents;
use sn_data_types::{
    ActorHistory, Credit, PublicKey, SignatureShare, SignedCredit, Token, TransferPropagated,
    WalletInfo,
};
use sn_messaging::{
    client::{Message, NodeCmd, NodeQuery, NodeRewardQuery, NodeSystemCmd},
    Aggregation, DstLocation, MessageId, SrcLocation,
};
use sn_routing::ElderKnowledge;
use std::collections::{BTreeMap, VecDeque};
use GenesisStage::*;

const GENESIS_ELDER_COUNT: usize = 5;

#[allow(clippy::large_enum_variant)]
enum Stage {
    Infant,
    Adult(AdultDuties),
    Genesis(GenesisStage),
    AssumingElderDuties((ElderState, VecDeque<ElderDuty>)),
    Elder(ElderConstellation),
}

#[allow(clippy::large_enum_variant)]
enum GenesisStage {
    AwaitingGenesisThreshold((ElderState, VecDeque<ElderDuty>)),
    ProposingGenesis(GenesisProposal),
    AccumulatingGenesis(GenesisAccumulation),
}

/// Node duties are those that all nodes
/// carry out. (TBD: adjust for Infant level, which might be doing nothing now).
/// Within the duty level, there are then additional
/// duties to be carried out, depending on the level.
pub struct NodeDuties {
    node_info: NodeInfo,
    stage: Stage,
    network_events: NetworkEvents,
    messaging: Messaging,
    network_api: Network,
}

/// Configuration made after connected to
/// network, or promoted to elder.
///
/// These are calls made as part of
/// a node initialising into a certain duty.
/// Being first node:
/// -> 1. Add own node id to rewards.
/// -> 2. Add own wallet to rewards.
/// Assuming Adult duties:
/// -> 1. Instantiate AdultDuties.
/// -> 2. Register wallet at Elders.
/// Assuming Elder duties:
/// -> 1. Instantiate ElderDuties.
/// -> 2. Add own node id to rewards.
/// -> 3. Add own wallet to rewards.
impl NodeDuties {
    pub async fn new(node_info: NodeInfo, network_api: Network) -> Result<Self> {
        let state = NodeState::Infant(network_api.public_key().await);
        let msg_analysis = ReceivedMsgAnalysis::new(state);
        let network_events = NetworkEvents::new(msg_analysis);
        let messaging = Messaging::new(network_api.clone());
        Ok(Self {
            node_info,
            stage: Stage::Infant,
            network_events,
            messaging,
            network_api,
        })
    }

    pub async fn process(&mut self, duty: NetworkDuty) -> Result<NetworkDuties> {
        use NetworkDuty::*;
        match duty {
            RunAsAdult(duty) => {
                if let Some(duties) = self.adult_duties() {
                    duties.process_adult_duty(duty).await
                } else {
                    Err(Error::Logic("Currently not an Adult".to_string()))
                }
            }
            RunAsElder(duty) => {
                if let Some(duties) = self.elder_duties() {
                    duties.process_elder_duty(duty).await
                } else if self.try_enqueue_elder_duty(duty) {
                    info!("Enqueued Elder duty");
                    Ok(vec![])
                } else {
                    Err(Error::Logic("Currently not an Elder".to_string()))
                }
            }
            RunAsNode(duty) => self.process_node_duty(duty).await,
            NoOp => Ok(vec![]),
        }
    }

    pub fn adult_duties(&mut self) -> Option<&mut AdultDuties> {
        use Stage::*;
        match &mut self.stage {
            Adult(ref mut duties) => Some(duties),
            _ => None,
        }
    }

    pub fn elder_duties(&mut self) -> Option<&mut ElderDuties> {
        match &mut self.stage {
            Stage::Elder(ref mut elder) => Some(elder.duties()),
            _ => None,
        }
    }

    pub fn try_enqueue_elder_duty(&mut self, duty: ElderDuty) -> bool {
        match self.stage {
            Stage::AssumingElderDuties((_, ref mut queue)) => {
                queue.push_back(duty);
                true
            }
            Stage::Genesis(ref mut stage) => match stage {
                AwaitingGenesisThreshold((_, ref mut queue)) => {
                    queue.push_back(duty);
                    true
                }
                ProposingGenesis(ref mut bootstrap) => {
                    bootstrap.queued_ops.push_back(duty);
                    true
                }
                AccumulatingGenesis(ref mut bootstrap) => {
                    bootstrap.queued_ops.push_back(duty);
                    true
                }
            },
            _ => false,
        }
    }

    fn node_state(&mut self) -> Result<NodeState> {
        Ok(match self.elder_duties() {
            Some(duties) => NodeState::Elder(duties.state().clone()),
            None => match self.adult_duties() {
                Some(duties) => NodeState::Adult(duties.state().clone()),
                None => {
                    return Err(Error::InvalidOperation(
                        "match self.adult_duties() is None".to_string(),
                    ))
                }
            },
        })
    }

    async fn process_node_duty(&mut self, duty: NodeDuty) -> Result<NetworkDuties> {
        use NodeDuty::*;
        info!("Processing Node duty: {:?}", duty);
        match duty {
            RegisterWallet(wallet) => self.register_wallet(wallet).await,
            AssumeAdultDuties => self.assume_adult_duties().await,
            AssumeElderDuties(elder_knowledge) => {
                self.begin_transition_to_elder(elder_knowledge).await
            }
            ReceiveGenesisProposal { credit, sig } => {
                self.receive_genesis_proposal(credit, sig).await
            }
            ReceiveGenesisAccumulation { signed_credit, sig } => {
                self.receive_genesis_accumulation(signed_credit, sig).await
            }
            InitiateElderChange(elder_knowledge) => {
                self.initiate_elder_change(elder_knowledge).await
            }
            FinishElderChange {
                previous_key,
                new_key,
            } => self.finish_elder_change(previous_key, new_key).await,
            InitSectionWallet(wallet_info) => {
                self.finish_transition_to_elder(wallet_info, None).await
            }
            ProcessMessaging(duty) => self.messaging.process_messaging_duty(duty).await,
            ProcessNetworkEvent(event) => {
                self.network_events
                    .process_network_event(event, &self.network_api)
                    .await
            }
            NoOp => Ok(vec![]),
            StorageFull => self.notify_section_of_our_storage().await,
        }
    }

    async fn notify_section_of_our_storage(&mut self) -> Result<NetworkDuties> {
        let node_id = PublicKey::from(self.network_api.public_key().await);
        Ok(NetworkDuties::from(NodeMessagingDuty::Send(OutgoingMsg {
            msg: Message::NodeCmd {
                cmd: NodeCmd::System(NodeSystemCmd::StorageFull {
                    section: node_id.into(),
                    node_id,
                }),
                id: MessageId::new(),
                target_section_pk: None,
            },
            dst: DstLocation::Section(node_id.into()),
            aggregation: Aggregation::None, // TODO: to_be_aggregated: Aggregation::AtDestination,
        })))
    }

    async fn register_wallet(&mut self, wallet: PublicKey) -> Result<NetworkDuties> {
        let node_state = self.node_state()?;
        info!("Registering wallet: {}", wallet);
        Ok(NetworkDuties::from(NodeMessagingDuty::Send(OutgoingMsg {
            msg: Message::NodeCmd {
                cmd: NodeCmd::System(NodeSystemCmd::RegisterWallet {
                    wallet,
                    section: PublicKey::Ed25519(node_state.node_id()).into(),
                }),
                id: MessageId::new(),
                target_section_pk: None,
            },
            dst: DstLocation::Section(wallet.into()),
            aggregation: Aggregation::None, // TODO: to_be_aggregated: Aggregation::AtDestination,
        })))
    }

    async fn assume_adult_duties(&mut self) -> Result<NetworkDuties> {
        if matches!(self.stage, Stage::Adult(_)) {
            return Ok(vec![]);
        }
        info!("Assuming Adult duties..");
        let node_id = self.network_api.public_key().await;
        let adult_reader = AdultReader::new(self.network_api.clone());
        let node_signing = NodeSigning::new(self.network_api.clone());
        let state = AdultState::new(node_id, adult_reader, node_signing).await?;
        let duties = AdultDuties::new(&self.node_info, state.clone()).await?;
        self.node_info.used_space.reset().await;
        self.stage = Stage::Adult(duties);
        self.network_events = NetworkEvents::new(ReceivedMsgAnalysis::new(NodeState::Adult(state)));
        info!("Adult duties assumed.");
        // NB: This is wrong, shouldn't write to disk here,
        // let it be upper layer resp.
        // Also, "Error-to-Unit" is not a good conversion..
        //dump_state(AgeGroup::Adult, self.node_info.path(), &self.id).unwrap_or(());
        Ok(NodeDuty::RegisterWallet(self.node_info.reward_key).into())
    }

    async fn begin_transition_to_elder(
        &mut self,
        elder_knowledge: ElderKnowledge,
    ) -> Result<NetworkDuties> {
        if matches!(self.stage, Stage::Elder(_))
            || matches!(self.stage, Stage::AssumingElderDuties(_))
            || matches!(self.stage, Stage::Genesis(AwaitingGenesisThreshold(_)))
        {
            return Ok(vec![]);
        } else if !self.node_info.genesis && matches!(self.stage, Stage::Infant) {
            return Err(Error::InvalidOperation(
                "only genesis node can transition to Elder as Infant".to_string(),
            ));
        }

        let is_genesis_section = self.network_api.our_prefix().await.is_empty();
        let elder_count = self.network_api.our_elder_names().await.len();
        let section_chain_len = self.network_api.section_chain().await.len();
        debug!(
            "begin_transition_to_elder. is_genesis_section: {}, elder_count: {}, section_chain_len: {}",
            is_genesis_section, elder_count, section_chain_len
        );

        let node_id = self.network_api.public_key().await;
        let dynamics = ElderDynamics::new(self.network_api.clone());
        let elder_state = ElderState::new(node_id, elder_knowledge, dynamics).await?;

        if is_genesis_section
            && elder_count == GENESIS_ELDER_COUNT
            && matches!(self.stage, Stage::Adult(_))
            && section_chain_len <= 5
        {
            // this is the case when we are the GENESIS_ELDER_COUNT-th Elder!
            debug!("threshold reached; proposing genesis!");
            let genesis_balance = u32::MAX as u64 * 1_000_000_000;
            let credit = Credit {
                id: Default::default(),
                amount: Token::from_nano(genesis_balance),
                recipient: elder_state.section_public_key(),
                msg: "genesis".to_string(),
            };
            let mut signatures: BTreeMap<usize, bls::SignatureShare> = Default::default();
            let credit_sig_share = elder_state.sign_as_elder(&credit).await?;
            let _ = signatures.insert(credit_sig_share.index, credit_sig_share.share.clone());

            self.stage = Stage::Genesis(ProposingGenesis(GenesisProposal {
                elder_state,
                proposal: credit.clone(),
                signatures,
                pending_agreement: None,
                queued_ops: VecDeque::new(),
            }));

            let dst = DstLocation::Section(credit.recipient.into());
            return Ok(NetworkDuties::from(NodeMessagingDuty::Send(OutgoingMsg {
                msg: Message::NodeCmd {
                    cmd: NodeCmd::System(NodeSystemCmd::ProposeGenesis {
                        credit,
                        sig: credit_sig_share,
                    }),
                    id: MessageId::new(),
                    target_section_pk: None,
                },
                dst,
                aggregation: Aggregation::None, // TODO: to_be_aggregated: Aggregation::AtDestination,
            })));
        } else if is_genesis_section && elder_count < GENESIS_ELDER_COUNT && section_chain_len <= 5
        {
            debug!("AwaitingGenesisThreshold!");
            self.stage = Stage::Genesis(AwaitingGenesisThreshold((elder_state, VecDeque::new())));
            return Ok(vec![]);
        }

        trace!("Beginning transition to Elder duties.");
        let wallet_key = elder_state.section_public_key();
        // must get the above wrapping instance before overwriting stage
        self.stage = Stage::AssumingElderDuties((elder_state, VecDeque::new()));
        // queries the other Elders for the section wallet history
        Ok(NetworkDuties::from(NodeMessagingDuty::Send(OutgoingMsg {
            msg: Message::NodeQuery {
                query: NodeQuery::Rewards(NodeRewardQuery::GetSectionWalletHistory),
                id: MessageId::new(),
                target_section_pk: None,
            },
            dst: DstLocation::Section(wallet_key.into()),
            aggregation: Aggregation::None, // TODO: to_be_aggregated: Aggregation::AtDestination,
        })))
    }

    // TODO: validate the credit...
    async fn receive_genesis_proposal(
        &mut self,
        credit: Credit,
        sig: SignatureShare,
    ) -> Result<NetworkDuties> {
        if matches!(self.stage, Stage::Genesis(AccumulatingGenesis(_)))
            || matches!(self.stage, Stage::Elder(_))
        {
            return Ok(vec![]);
        }

        let (stage, cmd) = match &mut self.stage {
            Stage::Genesis(AwaitingGenesisThreshold((elder_state, ref mut queued_ops))) => {
                let mut signatures: BTreeMap<usize, bls::SignatureShare> = Default::default();
                let _ = signatures.insert(sig.index, sig.share);

                let credit_sig_share = elder_state.sign_as_elder(&credit).await?;
                let _ = signatures.insert(credit_sig_share.index, credit_sig_share.share.clone());

                let dst = DstLocation::Section(elder_state.section_public_key().into());
                let stage = Stage::Genesis(ProposingGenesis(GenesisProposal {
                    elder_state: elder_state.to_owned(),
                    proposal: credit.clone(),
                    signatures,
                    pending_agreement: None,
                    queued_ops: queued_ops.drain(..).collect(),
                }));
                let cmd = NodeMessagingDuty::Send(OutgoingMsg {
                    msg: Message::NodeCmd {
                        cmd: NodeCmd::System(NodeSystemCmd::ProposeGenesis {
                            credit,
                            sig: credit_sig_share,
                        }),
                        id: MessageId::new(),
                        target_section_pk: None,
                    },
                    dst,
                    aggregation: Aggregation::None, // TODO: to_be_aggregated: Aggregation::AtDestination,
                });

                (stage, cmd)
            }
            Stage::Genesis(ProposingGenesis(ref mut bootstrap)) => {
                debug!("Adding incoming genesis proposal.");
                bootstrap.add(sig)?;
                if let Some(signed_credit) = &bootstrap.pending_agreement {
                    // replicas signatures over > signed_credit <
                    let mut signatures: BTreeMap<usize, bls::SignatureShare> = Default::default();
                    let credit_sig_share =
                        bootstrap.elder_state.sign_as_elder(&signed_credit).await?;
                    let _ =
                        signatures.insert(credit_sig_share.index, credit_sig_share.share.clone());

                    let stage = Stage::Genesis(AccumulatingGenesis(GenesisAccumulation {
                        elder_state: bootstrap.elder_state.clone(),
                        agreed_proposal: signed_credit.clone(),
                        signatures,
                        pending_agreement: None,
                        queued_ops: bootstrap.queued_ops.drain(..).collect(),
                    }));

                    let cmd = NodeMessagingDuty::Send(OutgoingMsg {
                        msg: Message::NodeCmd {
                            cmd: NodeCmd::System(NodeSystemCmd::AccumulateGenesis {
                                signed_credit: signed_credit.clone(),
                                sig: credit_sig_share,
                            }),
                            id: MessageId::new(),
                            target_section_pk: None,
                        },
                        dst: DstLocation::Section(
                            bootstrap.elder_state.section_public_key().into(),
                        ),
                        aggregation: Aggregation::None, // TODO: to_be_aggregated: Aggregation::AtDestination,
                    });

                    (stage, cmd)
                } else {
                    return Ok(vec![]);
                }
            }
            _ => {
                return Err(Error::InvalidOperation(
                    "invalid self.stage at fn receive_genesis_proposal".to_string(),
                ))
            }
        };

        self.stage = stage;

        Ok(NetworkDuties::from(cmd))
    }

    async fn receive_genesis_accumulation(
        &mut self,
        signed_credit: SignedCredit,
        sig: SignatureShare,
    ) -> Result<NetworkDuties> {
        if matches!(self.stage, Stage::Elder(_)) {
            return Ok(vec![]);
        }

        match self.stage {
            Stage::Genesis(ProposingGenesis(ref mut bootstrap)) => {
                // replicas signatures over > signed_credit <
                let mut signatures: BTreeMap<usize, bls::SignatureShare> = Default::default();
                let _ = signatures.insert(sig.index, sig.share);

                let credit_sig_share = bootstrap.elder_state.sign_as_elder(&signed_credit).await?;
                let _ = signatures.insert(credit_sig_share.index, credit_sig_share.share);

                self.stage = Stage::Genesis(AccumulatingGenesis(GenesisAccumulation {
                    elder_state: bootstrap.elder_state.clone(),
                    agreed_proposal: signed_credit,
                    signatures,
                    pending_agreement: None,
                    queued_ops: bootstrap.queued_ops.drain(..).collect(),
                }));
                Ok(vec![])
            }
            Stage::Genesis(AccumulatingGenesis(ref mut bootstrap)) => {
                bootstrap.add(sig)?;
                if let Some(genesis) = bootstrap.pending_agreement.take() {
                    // TODO: do not take this? (in case of fail further blow)
                    let credit_sig_share = bootstrap.elder_state.sign_as_elder(&genesis).await?;
                    let _ = bootstrap
                        .signatures
                        .insert(credit_sig_share.index, credit_sig_share.share.clone());

                    let genesis = TransferPropagated {
                        credit_proof: genesis.clone(),
                    };

                    debug!(">>> GENSIS AGREEMENT PROOFED");
                    return self
                        .finish_transition_to_elder(
                            WalletInfo {
                                replicas: genesis.credit_proof.debiting_replicas_keys.clone(),
                                history: ActorHistory {
                                    credits: vec![genesis.credit_proof.clone()],
                                    debits: vec![],
                                },
                            },
                            Some(genesis),
                        )
                        .await;
                }
                Ok(vec![])
            }
            _ => Err(Error::InvalidOperation(
                "invalid self.stage at fn receive_genesis_accumulation".to_string(),
            )),
        }
    }

    async fn finish_transition_to_elder(
        &mut self,
        wallet_info: WalletInfo,
        genesis: Option<TransferPropagated>,
    ) -> Result<NetworkDuties> {
        debug!(">>>Finishing transition to elder");
        let queued_ops = &mut VecDeque::new();
        let (elder_state, ref mut queued_duties) = match &mut self.stage {
            Stage::Elder(_) => return Ok(vec![]),
            Stage::Infant => {
                if self.node_info.genesis {
                    let node_id = self.network_api.public_key().await;
                    let dynamics = ElderDynamics::new(self.network_api.clone());
                    let state = ElderState::new(node_id, self.network_api.genesis_elder_knowledge().await?, dynamics).await?;
                    (state, queued_ops)
                } else {
                    return Err(Error::InvalidOperation("cannot finish_transition_to_elder as Infant".to_string()));
                }
            }
            Stage::Adult(_) | Stage::Genesis(AwaitingGenesisThreshold(_)) | Stage::Genesis(ProposingGenesis(_)) => {
                return Err(Error::InvalidOperation("cannot finish_transition_to_elder as Adult | AwaitingGenesisThreshold | ProposingGenesis".to_string()))
            }
            Stage::Genesis(AccumulatingGenesis(ref mut bootstrap)) => (bootstrap.elder_state.to_owned(), &mut bootstrap.queued_ops),
            Stage::AssumingElderDuties((elder_state, queue)) => (elder_state.to_owned(), queue),
        };

        trace!("Finishing transition to Elder..");
        // NB: still snapshotting here

        let mut ops: NetworkDuties = vec![];

        let mut duties =
            ElderDuties::new(wallet_info, &self.node_info, elder_state.clone()).await?;

        // 1. Initiate duties.
        ops.extend(duties.initiate(genesis).await?);

        // 2. Process all enqueued duties.
        for duty in queued_duties.drain(..) {
            debug!("queued duty: {:?}", duty);
            ops.extend(duties.process_elder_duty(duty).await?);
        }

        // 3. Set new stage
        self.node_info.used_space.reset().await;
        self.stage = Stage::Elder(ElderConstellation::new(duties));
        self.network_events = NetworkEvents::new(ReceivedMsgAnalysis::new(NodeState::Elder(
            elder_state.clone(),
        )));
        // NB: This is wrong, shouldn't write to disk here,
        // let it be upper layer resp.
        // Also, "Error-to-Unit" is not a good conversion..
        //dump_state(AgeGroup::Elder, self.node_info.path(), &self.id).unwrap_or(())

        info!("Successfully assumed Elder duties!");

        let node_id = elder_state.node_name();

        // 4. Add own node id to rewards.
        ops.push(NetworkDuty::from(RewardDuty::ProcessCmd {
            cmd: RewardCmd::AddNewNode(node_id),
            msg_id: MessageId::new(),
            origin: SrcLocation::Node(node_id),
        }));

        // 5. Add own wallet to rewards.
        ops.push(NetworkDuty::from(RewardDuty::ProcessCmd {
            cmd: RewardCmd::SetNodeWallet {
                node_id,
                wallet_id: self.node_info.reward_key,
            },
            msg_id: MessageId::new(),
            origin: SrcLocation::Node(node_id),
        }));

        Ok(ops)
    }

    ///
    async fn initiate_elder_change(
        &mut self,
        elder_knowledge: ElderKnowledge,
    ) -> Result<NetworkDuties> {
        match &mut self.stage {
            Stage::Infant | Stage::AssumingElderDuties(_) | Stage::Genesis(_) | Stage::Adult(_) => {
                Ok(vec![])
            }
            Stage::Elder(elder) => elder.initiate_elder_change(elder_knowledge).await,
        }
    }

    ///
    async fn finish_elder_change(
        &mut self,
        previous_key: PublicKey,
        new_key: PublicKey,
    ) -> Result<NetworkDuties> {
        match &mut self.stage {
            Stage::Infant | Stage::Adult(_) | Stage::Genesis(_) | Stage::AssumingElderDuties(_) => {
                Ok(vec![])
            } // Should be unreachable
            Stage::Elder(elder) => {
                elder
                    .finish_elder_change(&self.node_info, previous_key, new_key)
                    .await
            }
        }
    }
}
