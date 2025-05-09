use std::io::{Error, ErrorKind, Read, Write};

use arbitrary::{Arbitrary, Unstructured};
use bitcoin::{
    absolute::LockTime,
    consensus::{deserialize, serialize},
    hashes::Hash,
    transaction::Version,
    Amount, ScriptBuf, Transaction, Txid, Witness,
};
use borsh::{BorshDeserialize, BorshSerialize};

/// [Borsh](borsh)-friendly Bitcoin [`Transaction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitcoinTx(Transaction);

impl From<Transaction> for BitcoinTx {
    fn from(value: Transaction) -> Self {
        Self(value)
    }
}

impl From<BitcoinTx> for Transaction {
    fn from(value: BitcoinTx) -> Self {
        value.0
    }
}

impl AsRef<Transaction> for BitcoinTx {
    fn as_ref(&self) -> &Transaction {
        &self.0
    }
}

/// Implement BorshSerialize using Bitcoin consensus serialization.
impl BorshSerialize for BitcoinTx {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Use bitcoin's consensus serialization
        let tx_bytes = serialize(&self.0);
        BorshSerialize::serialize(&(tx_bytes.len() as u32), writer)?;
        writer.write_all(&tx_bytes)
    }
}

/// Implement BorshDeserialize using Bitcoin consensus deserialization.
impl BorshDeserialize for BitcoinTx {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        // First, read a Vec<u8> using Borsh (this picks up the length)
        let tx_len = u32::deserialize_reader(reader)? as usize;
        let mut tx_bytes = vec![0u8; tx_len];
        reader.read_exact(&mut tx_bytes)?;

        // Now parse those bytes with bitcoin consensus
        let tx = deserialize(&tx_bytes).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

        Ok(BitcoinTx(tx))
    }
}

impl<'a> Arbitrary<'a> for BitcoinTx {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        use bitcoin::{
            blockdata::transaction::{OutPoint, TxIn, TxOut},
            Sequence, Transaction,
        };

        // Random number of inputs and outputs (bounded for simplicity)
        let input_count = u.int_in_range::<usize>(0..=4)?;
        let output_count = u.int_in_range::<usize>(0..=4)?;

        // Build random inputs
        let mut inputs = Vec::with_capacity(input_count);
        for _ in 0..input_count {
            // Random 32-byte TXID
            let mut txid_bytes = [0u8; 32];
            u.fill_buffer(&mut txid_bytes)?;

            // Random vout
            let vout = u32::arbitrary(u)?;

            // Random scriptSig (bounded size)
            let script_sig_size = u.int_in_range::<usize>(0..=50)?;
            let script_sig_bytes = u.bytes(script_sig_size)?;
            let script_sig = ScriptBuf::from_bytes(script_sig_bytes.to_vec());

            inputs.push(TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array(txid_bytes),
                    vout,
                },
                script_sig,
                sequence: Sequence::MAX,
                witness: Witness::default(), // or generate random witness if desired
            });
        }

        // Build random outputs
        let mut outputs = Vec::with_capacity(output_count);
        for _ in 0..output_count {
            // Random value (in satoshis)
            let value = Amount::from_sat(u64::arbitrary(u)?);

            // Random scriptPubKey (bounded size)
            let script_pubkey_size = u.int_in_range::<usize>(0..=50)?;
            let script_pubkey_bytes = u.bytes(script_pubkey_size)?;
            let script_pubkey = ScriptBuf::from(script_pubkey_bytes.to_vec());

            outputs.push(TxOut {
                value,
                script_pubkey,
            });
        }

        // Construct the transaction
        let tx = Transaction {
            version: Version::ONE,
            lock_time: LockTime::ZERO,
            input: inputs,
            output: outputs,
        };

        Ok(BitcoinTx(tx))
    }
}

#[cfg(test)]
mod tests {
    use strata_bridge_test_utils::arbitrary_generator::ArbitraryGenerator;

    use super::*;

    #[test]
    fn test_bitcoin_tx_serialize_deserialize() {
        let mut generator = ArbitraryGenerator::new();
        let tx: BitcoinTx = generator.generate();

        let serialized_tx = borsh::to_vec(&tx).expect("should be able to serialize BitcoinTx");
        let deserialized_tx: BitcoinTx =
            borsh::from_slice(&serialized_tx).expect("should be able to deserialize BitcoinTx");

        assert_eq!(
            tx, deserialized_tx,
            "original and deserialized tx must be the same"
        );
    }
}
