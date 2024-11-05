use bitcoin::{Amount, Network, Txid};
use secp256k1::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use strata_bridge_db::connector_db::ConnectorDb;
use strata_bridge_primitives::{
    build_context::BuildContext,
    params::connectors::{
        NUM_PKS_A160, NUM_PKS_A160_PER_CONNECTOR, NUM_PKS_A256, NUM_PKS_A256_PER_CONNECTOR,
    },
    types::OperatorIdx,
};

use crate::{connectors::prelude::*, transactions::prelude::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PegOutGraphInput {
    pub network: Network,

    pub deposit_amount: Amount,

    pub operator_pubkey: XOnlyPublicKey,

    pub kickoff_data: KickoffTxData,
}

#[derive(Debug, Clone)]
pub struct PegOutGraph {
    pub kickoff_tx: KickOffTx,

    pub claim_tx: ClaimTx,

    pub assert_chain: AssertChain,

    pub payout_tx: PayoutTx,

    pub disprove_tx: DisproveTx,
}

impl PegOutGraph {
    pub async fn generate<Db: ConnectorDb>(
        input: PegOutGraphInput,
        deposit_txid: Txid,
        connectors: PegOutGraphConnectors<Db>,
    ) -> Self {
        let kickoff_tx = KickOffTx::new(
            input.kickoff_data,
            connectors.kickoff.clone(),
            input.network,
        )
        .await;
        let kickoff_txid = kickoff_tx.compute_txid();

        let claim_data = ClaimData {
            kickoff_txid,
            deposit_txid,
        };

        let claim_tx = ClaimTx::new(
            claim_data,
            connectors.kickoff,
            connectors.claim_out_0,
            connectors.claim_out_1,
        )
        .await;
        let claim_txid = claim_tx.compute_txid();

        let assert_chain_data = AssertChainData {
            pre_assert_data: PreAssertData {
                claim_txid,
                input_stake: claim_tx.remaining_stake(),
            },
            deposit_txid,
        };

        let assert_chain = AssertChain::new(
            assert_chain_data,
            connectors.claim_out_0,
            connectors.stake,
            connectors.post_assert_out_0.clone(),
            connectors.post_assert_out_1.clone(),
            connectors.assert_data160_factory,
            connectors.assert_data256_factory,
        )
        .await;

        let post_assert_txid = assert_chain.post_assert.compute_txid();
        let post_assert_out_stake = assert_chain.post_assert.remaining_stake();

        let payout_data = PayoutData {
            post_assert_txid,
            deposit_txid,
            input_stake: post_assert_out_stake,
            deposit_amount: input.deposit_amount,
            operator_key: input.operator_pubkey,
            network: input.network,
        };

        let payout_tx = PayoutTx::new(
            payout_data,
            connectors.post_assert_out_0.clone(),
            connectors.stake,
        );

        let disprove_data = DisproveData {
            post_assert_txid,
            deposit_txid,
            input_stake: post_assert_out_stake,
            network: input.network,
        };

        let disprove_tx = DisproveTx::new(
            disprove_data,
            connectors.post_assert_out_0,
            connectors.post_assert_out_1,
        )
        .await;

        Self {
            kickoff_tx,
            claim_tx,
            assert_chain,
            payout_tx,
            disprove_tx,
        }
    }
}

#[derive(Debug)]
pub struct PegOutGraphConnectors<Db: ConnectorDb + Clone> {
    pub kickoff: ConnectorK<Db>,

    pub claim_out_0: ConnectorC0,

    pub claim_out_1: ConnectorC1,

    pub stake: ConnectorS,

    pub post_assert_out_0: ConnectorA30<Db>,

    pub post_assert_out_1: ConnectorA31<Db>,

    pub assert_data160_factory: ConnectorA160Factory<NUM_PKS_A160_PER_CONNECTOR, NUM_PKS_A160>,

    pub assert_data256_factory: ConnectorA256Factory<NUM_PKS_A256_PER_CONNECTOR, NUM_PKS_A256>,
}

impl<Db: ConnectorDb> PegOutGraphConnectors<Db> {
    pub async fn new(
        db: Db,
        build_context: &impl BuildContext,
        deposit_txid: Txid,
        operator_idx: OperatorIdx,
    ) -> Self {
        let n_of_n_agg_pubkey = build_context.aggregated_pubkey();
        let network = build_context.network();

        let kickoff = ConnectorK::new(n_of_n_agg_pubkey, network, operator_idx, db.clone());

        let claim_out_0 = ConnectorC0::new(n_of_n_agg_pubkey, network);

        let claim_out_1 = ConnectorC1::new(n_of_n_agg_pubkey, network);

        let stake = ConnectorS::new(n_of_n_agg_pubkey, network);

        let post_assert_out_0 = ConnectorA30::new(n_of_n_agg_pubkey, network, db.clone());
        let post_assert_out_1 = ConnectorA31::new(network, db.clone());

        let ([_, _, superblock_hash_public_key], public_keys_256, public_keys_160) =
            db.get_wots_public_keys(operator_idx, deposit_txid).await;
        let assert_data160_factory: ConnectorA160Factory<NUM_PKS_A160_PER_CONNECTOR, NUM_PKS_A160> =
            ConnectorA160Factory {
                network,
                public_keys: public_keys_160,
            };

        let public_keys_256 = std::array::from_fn(|i| {
            if i == 0 {
                superblock_hash_public_key
            } else {
                public_keys_256[i - 1]
            }
        });

        let assert_data256_factory: ConnectorA256Factory<NUM_PKS_A256_PER_CONNECTOR, NUM_PKS_A256> =
            ConnectorA256Factory {
                network,
                public_keys: public_keys_256,
            };

        Self {
            kickoff,
            claim_out_0,
            claim_out_1,
            stake,
            post_assert_out_0,
            post_assert_out_1,
            assert_data160_factory,
            assert_data256_factory,
        }
    }
}
