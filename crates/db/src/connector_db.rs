use std::fmt::Debug;

use async_trait::async_trait;
use bitcoin::Txid;
use bitvm::{
    groth16::g16::{self, N_TAPLEAVES},
    treepp::*,
};
use secp256k1::schnorr::Signature;
use strata_bridge_primitives::types::OperatorIdx;

#[async_trait]
pub trait ConnectorDb: Clone + Debug + Send + Sync {
    async fn get_verifier_scripts(&self) -> [Script; N_TAPLEAVES];

    async fn get_wots_public_keys(
        &self,
        operator_id: u32,
        deposit_txid: Txid,
    ) -> g16::WotsPublicKeys;

    async fn set_wots_public_keys(
        &self,
        operator_id: u32,
        deposit_txid: Txid,
        public_keys: &g16::WotsPublicKeys,
    );

    async fn get_wots_signatures(
        &self,
        operator_id: u32,
        deposit_txid: Txid,
    ) -> g16::WotsSignatures;

    async fn set_wots_signatures(
        &self,
        operator_id: u32,
        deposit_txid: Txid,
        signatures: &g16::WotsSignatures,
    );

    async fn get_signature(
        &self,
        operator_idx: OperatorIdx,
        txid: Txid,
        input_index: u32,
    ) -> Signature;
}
