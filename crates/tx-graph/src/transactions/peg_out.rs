use bitcoin::{
    absolute, consensus, Amount, EcdsaSighashType, Network, PublicKey, ScriptBuf, Sequence,
    Transaction, TxIn, TxOut, Witness,
};
use serde::{Deserialize, Serialize};
use strata_bridge_contexts::operator::OperatorContext;

use super::{
    super::{graphs::base::FEE_AMOUNT, scripts::*},
    base::*,
    pre_signed::*,
};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct PegOutTransaction {
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    tx: Transaction,
    #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
    prev_outs: Vec<TxOut>,
    prev_scripts: Vec<ScriptBuf>,
}

impl PreSignedTransaction for PegOutTransaction {
    fn tx(&self) -> &Transaction {
        &self.tx
    }

    fn tx_mut(&mut self) -> &mut Transaction {
        &mut self.tx
    }

    fn prev_outs(&self) -> &Vec<TxOut> {
        &self.prev_outs
    }

    fn prev_scripts(&self) -> &Vec<ScriptBuf> {
        &self.prev_scripts
    }
}

impl PegOutTransaction {
    pub fn new(
        context: &OperatorContext,
        withdrawer_public_key: &PublicKey,
        evm_address: &str,
        evm_peg_out_ts: u32,
        input_0: Input,
    ) -> Self {
        let mut this = Self::new_for_validation(
            context.network,
            &context.operator_public_key,
            withdrawer_public_key,
            evm_address,
            evm_peg_out_ts,
            input_0,
        );

        this.sign_input_0(context);

        this
    }

    pub fn new_for_validation(
        network: Network,
        operator_public_key: &PublicKey,
        withdrawer_public_key: &PublicKey,
        evm_address: &str,
        evm_peg_out_ts: u32,
        input_0: Input,
    ) -> Self {
        let _input_0 = TxIn {
            previous_output: input_0.outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::default(),
        };

        let total_output_amount = input_0.amount - Amount::from_sat(FEE_AMOUNT);

        let _output_0 = TxOut {
            value: total_output_amount,
            script_pubkey: generate_pay_to_pubkey_hash_with_inscription_script_address(
                network,
                withdrawer_public_key,
                evm_peg_out_ts,
                evm_address,
            )
            .script_pubkey(),
        };

        PegOutTransaction {
            tx: Transaction {
                version: bitcoin::transaction::Version(2),
                lock_time: absolute::LockTime::ZERO,
                input: vec![_input_0],
                output: vec![_output_0],
            },
            prev_outs: vec![TxOut {
                value: input_0.amount,
                script_pubkey: generate_pay_to_pubkey_script_address(network, operator_public_key)
                    .script_pubkey(),
            }],
            prev_scripts: vec![generate_pay_to_pubkey_script(operator_public_key)],
        }
    }

    fn sign_input_0(&mut self, context: &OperatorContext) {
        let input_index = 0;
        pre_sign_p2wsh_input(
            self,
            context,
            input_index,
            EcdsaSighashType::All,
            &vec![&context.operator_keypair],
        );
    }
}

impl BaseTransaction for PegOutTransaction {
    fn finalize(&self) -> Transaction {
        self.tx.clone()
    }
}
