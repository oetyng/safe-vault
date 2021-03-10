// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    node::node_ops::{
        AdultDuty, ChunkReplicationCmd, ChunkReplicationDuty, ChunkReplicationQuery,
        ChunkStoreDuty, ElderDuty, MetadataDuty, NetworkDuties, NodeDuty, RewardCmd, RewardDuty,
        RewardQuery, TransferCmd, TransferDuty, TransferQuery,
    },
    AdultState, Error, NodeState, Result,
};
use log::{debug, info};
use sn_messaging::{
    client::{
        Cmd, Message, NodeCmd, NodeDataQueryResponse, NodeEvent, NodeQuery, NodeQueryResponse,
        NodeRewardQuery, NodeRewardQueryResponse, NodeSystemCmd, NodeSystemQuery,
        NodeSystemQueryResponse, NodeTransferCmd, NodeTransferQuery, NodeTransferQueryResponse,
        Query,
    },
    DstLocation, EndUser, MessageId, SrcLocation,
};
use sn_routing::XorName;

/// Evaluates remote msgs from the network,
/// i.e. not msgs sent directly from a client.
// pub struct HandleMessage {
//     state: NodeState,
// }

// impl HandleMessage {
    // pub fn new(state: NodeState) -> Self {
    //     Self { state }
    // }

    // pub fn name(&self) -> XorName {
    //     self.state.node_name()
    // }

    pub fn handle_msg(
        msg: Message,
        src: SrcLocation,
        dst: DstLocation,
    ) -> Result<()> {
        debug!(">>>>>>>>>>>> Evaluating received msg. {:?}.", msg);
        let msg_id = msg.id();
        if let SrcLocation::EndUser(origin) = src {
            match_user_sent_msg(msg.clone(), origin)?
            // if res.is_empty() {
            //     return Err(Error::InvalidMessage(
            //         msg_id,
            //         format!("No match for user msg: {:?}", msg),
            //     ));
            // }
            // return Ok()
        }
        if let DstLocation::EndUser(_dst) = dst {
            unimplemented!()
        }

        match &dst {
            DstLocation::Section(_name) => {
                match_section_msg(msg.clone(), src)
                // if res.is_empty() {
                //     match_node_msg(msg, src)
                // } else {
                //     Ok(res)
                // }
            }
            DstLocation::Node(_name) => {
                match_node_msg(msg.clone(), src)
                // if res.is_empty() {
                //     match_section_msg(msg, src)
                // } else {
                //     Ok(res)
                // }
            }
            _ => Err(Error::InvalidMessage(
                msg_id,
                format!("Invalid dst: {:?}", msg),
            )),
        }
    }

    fn match_user_sent_msg( msg: Message, origin: EndUser) -> Result<()> {
        match msg {
            // TODO: match and parse directly
            // Message::Query {
            //     query: Query::Data(query),
            //     id,
            //     ..
            // } => NetworkDuties::from(MetadataDuty::ProcessRead { query, id, origin }),
            // Message::Cmd {
            //     cmd: Cmd::Data { .. },
            //     id,
            //     ..
            // } => NetworkDuties::from(TransferDuty::ProcessCmd {
            //     cmd: TransferCmd::ProcessPayment(msg.clone()),
            //     msg_id: id,
            //     origin: SrcLocation::EndUser(origin),
            // }),
            // Message::Cmd {
            //     cmd: Cmd::Transfer(cmd),
            //     id,
            //     ..
            // } => NetworkDuties::from(TransferDuty::ProcessCmd {
            //     cmd: cmd.into(),
            //     msg_id: id,
            //     origin: SrcLocation::EndUser(origin),
            // }),
            // Message::Query {
            //     query: Query::Transfer(query),
            //     id,
            //     ..
            // } => NetworkDuties::from(TransferDuty::ProcessQuery {
            //     query: query.into(),
            //     msg_id: id,
            //     origin: SrcLocation::EndUser(origin),
            // }),
            _ => Ok(()),
        }
    }

    fn match_section_msg( msg: Message, origin: SrcLocation) -> Result<()> {
        debug!("Evaluating section message: {:?}", msg);

        match &msg {
            //
            // ------ metadata ------
            // Message::NodeQuery {
            //     query: NodeQuery::Metadata { query, origin },
            //     id,
            //     ..
            // } => MetadataDuty::ProcessRead {
            //     query: query.clone(),
            //     id: *id,
            //     origin: *origin,
            // }
            // .into(),
            // Message::NodeCmd {
            //     cmd: NodeCmd::Metadata { cmd, origin },
            //     id,
            //     ..
            // } => MetadataDuty::ProcessWrite {
            //     cmd: cmd.clone(),
            //     id: *id,
            //     origin: *origin,
            // }
            // .into(),
            // //
            // // ------ adult ------
            // Message::NodeQuery {
            //     query: NodeQuery::Chunks { query, origin },
            //     id,
            //     ..
            // } => AdultDuty::RunAsChunkStore(ChunkStoreDuty::ReadChunk {
            //     read: query.clone(),
            //     id: *id,
            //     origin: *origin,
            // })
            // .into(),
            // Message::NodeCmd {
            //     cmd: NodeCmd::Chunks { cmd, origin },
            //     id,
            //     ..
            // } => AdultDuty::RunAsChunkStore(ChunkStoreDuty::WriteChunk {
            //     write: cmd.clone(),
            //     id: *id,
            //     origin: *origin,
            // })
            // .into(),
            // //
            // // ------ chunk replication ------
            // Message::NodeQuery {
            //     query:
            //         NodeQuery::System(NodeSystemQuery::GetChunk {
            //             //section_authority,
            //             new_holder,
            //             address,
            //             current_holders,
            //         }),
            //     ..
            // } => {
            //     info!("Verifying GetChunk query!");
            //     let _proof_chain = self.adult_state()?.section_chain();

            //     // Recreate original MessageId from Section
            //     let msg_id = MessageId::combine(vec![*address.name(), *new_holder]);

            //     // Recreate cmd that was sent by the section.
            //     let _message = Message::NodeCmd {
            //         cmd: NodeCmd::System(NodeSystemCmd::ReplicateChunk {
            //             new_holder: *new_holder,
            //             address: *address,
            //             current_holders: current_holders.clone(),
            //         }),
            //         id: msg_id,
            //         target_section_pk: None,
            //     };

            //     info!("Internal ChunkReplicationQuery ProcessQuery");
            //     AdultDuty::RunAsChunkReplication(ChunkReplicationDuty::ProcessQuery {
            //         query: ChunkReplicationQuery::GetChunk(*address),
            //         msg_id,
            //         origin,
            //     })
            //     .into()
            // }
            // // this cmd is accumulated, thus has authority
            // Message::NodeCmd {
            //     cmd:
            //         NodeCmd::System(NodeSystemCmd::ReplicateChunk {
            //             address,
            //             current_holders,
            //             ..
            //         }),
            //     id,
            //     ..
            // } => AdultDuty::RunAsChunkReplication(ChunkReplicationDuty::ProcessCmd {
            //     cmd: ChunkReplicationCmd::ReplicateChunk {
            //         current_holders: current_holders.clone(),
            //         address: *address,
            //     },
            //     msg_id: *id,
            //     origin,
            // })
            // .into(),
            // //
            // // ------ Rewards ------
            // Message::NodeQuery {
            //     query:
            //         NodeQuery::Rewards(NodeRewardQuery::GetNodeWalletId {
            //             old_node_id,
            //             new_node_id,
            //         }),
            //     id,
            //     ..
            // } => RewardDuty::ProcessQuery {
            //     query: RewardQuery::GetNodeWalletId {
            //         old_node_id: *old_node_id,
            //         new_node_id: *new_node_id,
            //     },
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // // trivial to accumulate
            // Message::NodeQueryResponse {
            //     response:
            //         NodeQueryResponse::Rewards(NodeRewardQueryResponse::GetNodeWalletId(Ok((
            //             wallet_id,
            //             new_node_id,
            //         )))),
            //     id,
            //     ..
            // } => RewardDuty::ProcessCmd {
            //     cmd: RewardCmd::ActivateNodeRewards {
            //         id: *wallet_id,
            //         node_id: *new_node_id,
            //     },
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // //
            // // ------ transfers --------
            // // doesn't need to be accumulated, but makes it a bit slimmer..
            // Message::NodeCmd {
            //     cmd: NodeCmd::Transfers(NodeTransferCmd::PropagateTransfer(proof)),
            //     id,
            //     ..
            // } => TransferDuty::ProcessCmd {
            //     cmd: TransferCmd::PropagateTransfer(proof.credit_proof()),
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // // tricky to accumulate, since it has a vec of events.. but we try anyway for now..
            // Message::NodeQueryResponse {
            //     response:
            //         NodeQueryResponse::Transfers(NodeTransferQueryResponse::GetReplicaEvents(events)),
            //     id,
            //     ..
            // } => TransferDuty::ProcessCmd {
            //     cmd: TransferCmd::InitiateReplica(events.clone()?),
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // // doesn't need to be accumulated, but makes it a bit slimmer..
            // Message::NodeCmd {
            //     cmd: NodeCmd::Transfers(NodeTransferCmd::RegisterSectionPayout(debit_agreement)),
            //     id,
            //     ..
            // } => TransferDuty::ProcessCmd {
            //     cmd: TransferCmd::RegisterSectionPayout(debit_agreement.clone()),
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // // Aggregated by us, for security
            // Message::NodeQuery {
            //     query: NodeQuery::System(NodeSystemQuery::GetSectionPkSet),
            //     id,
            //     ..
            // } => NodeDuty::GetSectionPkSet {
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // Message::NodeEvent {
            //     event: NodeEvent::SectionPayoutRegistered { from, to },
            //     ..
            // } => NodeDuty::CompleteElderChange {
            //     previous_key: *from,
            //     new_key: *to,
            // }
            // .into(),
            // Message::NodeEvent {
            //     event:
            //         NodeEvent::PromotedToElder {
            //             section_wallet,
            //             node_rewards,
            //             user_wallets,
            //         },
            //     ..
            // } => NodeDuty::CompleteTransitionToElder {
            //     section_wallet: section_wallet.to_owned(),
            //     node_rewards: node_rewards.to_owned(),
            //     user_wallets: user_wallets.to_owned(),
            // }
            // .into(),
            _ => Ok(()),
        }
    }

    fn match_node_msg(msg: Message, origin: SrcLocation) -> Result<()> {
        debug!("Evaluating node node: {:?}", msg);

        match &msg {
            //
            // ------ wallet register ------
            // Message::NodeCmd {
            //     cmd: NodeCmd::System(NodeSystemCmd::RegisterWallet(wallet)),
            //     id,
            //     ..
            // } => RewardDuty::ProcessCmd {
            //     cmd: RewardCmd::SetNodeWallet {
            //         wallet_id: *wallet,
            //         node_id: origin.to_dst().name().ok_or_else(|| {
            //             Error::InvalidMessage(*id, "Missing origin name!".to_string())
            //         })?,
            //     },
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // //
            // // ------ system cmd ------
            // Message::NodeCmd {
            //     cmd: NodeCmd::System(NodeSystemCmd::StorageFull { node_id, .. }),
            //     ..
            // } => ElderDuty::StorageFull { node_id: *node_id }.into(),
            // //
            // // ------ node duties ------
            // Message::NodeCmd {
            //     cmd: NodeCmd::System(NodeSystemCmd::ProposeGenesis { credit, sig }),
            //     ..
            // } => NodeDuty::ReceiveGenesisProposal {
            //     credit: credit.clone(),
            //     sig: sig.clone(),
            // }
            // .into(),
            // Message::NodeCmd {
            //     cmd: NodeCmd::System(NodeSystemCmd::AccumulateGenesis { signed_credit, sig }),
            //     ..
            // } => NodeDuty::ReceiveGenesisAccumulation {
            //     signed_credit: signed_credit.clone(),
            //     sig: sig.clone(),
            // }
            // .into(),
            // //
            // // ------ chunk replication ------
            // // query response from adult cannot be accumulated
            // Message::NodeQueryResponse {
            //     response: NodeQueryResponse::Data(NodeDataQueryResponse::GetChunk(result)),
            //     correlation_id,
            //     ..
            // } => {
            //     let blob = result.to_owned()?;
            //     info!("Verifying GetChunk NodeQueryResponse!");
            //     // Recreate original MessageId from Section
            //     let msg_id =
            //         MessageId::combine(vec![*blob.address().name(), self.state.node_name()]);
            //     if msg_id == *correlation_id {
            //         AdultDuty::RunAsChunkReplication(ChunkReplicationDuty::ProcessCmd {
            //             cmd: ChunkReplicationCmd::StoreReplicatedBlob(blob),
            //             msg_id,
            //             origin,
            //         })
            //         .into()
            //     } else {
            //         info!("Given blob is incorrect.");
            //         panic!()
            //     }
            // }
            // //
            // // ------ nonacc rewards ------
            // // validated event cannot be accumulated at routing, since it has sig shares
            // Message::NodeEvent {
            //     event: NodeEvent::SectionPayoutValidated(validation),
            //     id,
            //     ..
            // } => RewardDuty::ProcessCmd {
            //     cmd: RewardCmd::ReceivePayoutValidation(validation.clone()),
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // //
            // // ------ nonacc transfers ------
            // // queries are from single source, so cannot be accumulated
            // Message::NodeQuery {
            //     query: NodeQuery::Transfers(NodeTransferQuery::GetReplicaEvents),
            //     id,
            //     ..
            // } => TransferDuty::ProcessQuery {
            //     query: TransferQuery::GetReplicaEvents,
            //     msg_id: *id,
            //     origin,
            // }
            // .into(),
            // // cannot be accumulated due to having sig share
            // Message::NodeCmd {
            //     cmd: NodeCmd::Transfers(NodeTransferCmd::ValidateSectionPayout(signed_transfer)),
            //     id,
            //     ..
            // } => {
            //     debug!(">>>> validating section payout to {:?}", signed_transfer);
            //     TransferDuty::ProcessCmd {
            //         cmd: TransferCmd::ValidateSectionPayout(signed_transfer.clone()),
            //         msg_id: *id,
            //         origin,
            //     }
            //     .into()
            // }
            // // // from a single src, so cannot be accumulated
            // // Message::NodeQuery {
            // //     query: NodeQuery::Rewards(NodeRewardQuery::GetSectionWalletHistory),
            // //     id,
            // //     ..
            // // } => RewardDuty::ProcessQuery {
            // //     query: RewardQuery::GetSectionWalletHistory,
            // //     msg_id: *id,
            // //     origin,
            // // }
            // // .into(),
            // // --- Adult ---
            // Message::NodeQuery {
            //     query: NodeQuery::Chunks { query, origin },
            //     id,
            //     ..
            // } => AdultDuty::RunAsChunkStore(ChunkStoreDuty::ReadChunk {
            //     read: query.clone(),
            //     id: *id,
            //     origin: *origin,
            // })
            // .into(),
            // Message::NodeCmd {
            //     cmd: NodeCmd::Chunks { cmd, origin },
            //     id,
            //     ..
            // } => AdultDuty::RunAsChunkStore(ChunkStoreDuty::WriteChunk {
            //     write: cmd.clone(),
            //     id: *id,
            //     origin: *origin,
            // })
            // .into(),
            // // tricky to accumulate, since it has a vec of events.. but we try anyway for now..
            // Message::NodeQueryResponse {
            //     response:
            //         NodeQueryResponse::System(NodeSystemQueryResponse::GetSectionPkSet(replicas)),
            //     id,
            //     ..
            // } => {
            //     debug!(">>>>> Should be handling CompleteWalletTransition, after GetSectionPkSet query response");
            //     RewardDuty::ProcessCmd {
            //         cmd: RewardCmd::CompleteWalletTransition(replicas.to_owned()),
            //         msg_id: *id,
            //         origin,
            //     }
            //     .into()
            // }
            _ => Ok(()),
        }
    }

    // fn adult_state(&self) -> Result<&AdultState> {
    //     if let NodeState::Adult(state) = &self.state {
    //         Ok(state)
    //     } else {
    //         Err(Error::InvalidOperation(
    //             "Tried to get adult state when there was none.".to_string(),
    //         ))
    //     }
    // }
// }
