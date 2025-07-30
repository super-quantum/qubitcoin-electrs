use crate::chain::BlockHeader;
use bitcoin::{consensus::Encodable, hashes::Hash, Block, BlockHash};
use fixed::types::I1F15;
use qip::prelude::*;
use sha2::{Digest, Sha256};
use state_ops::measurement_ops::measure_prob;
use std::f64::consts::PI;

pub trait QHashable {
    fn qhash(&self) -> BlockHash;
}

impl QHashable for BlockHeader {
    fn qhash(&self) -> BlockHash {
        let mut header = Vec::<u8>::new();
        self.consensus_encode(&mut header).unwrap();

        let hashed: [u8; 32] = Sha256::digest(header).into();
        let expectations = run_simulation(&hashed);
        let values = expectations
            .into_iter()
            .flat_map(|prob| I1F15::from_num(prob).to_le_bytes())
            .collect::<Vec<_>>();
        let mut final_data = hashed.to_vec();
        final_data.extend(values.into_iter());

        let hash: [u8; 32] = Sha256::digest(final_data).into();
        BlockHash::from_byte_array(hash)
    }
}

impl QHashable for Block {
    fn qhash(&self) -> BlockHash {
        self.header.qhash()
    }
}

const NUM_LAYERS: usize = 2;
const NUM_QUBITS: usize = 16;

fn run_simulation(data: &[u8; 32]) -> Vec<f64> {
    let mut b = LocalBuilder::<f64>::default();
    let mut r: [_; NUM_QUBITS] = core::array::from_fn(|_| Some(b.qubit()));

    for l in 0..NUM_LAYERS {
        for i in 0..r.len() {
            let byte_y = data[l * NUM_QUBITS + i / 2];
            let nibble_y = if i % 2 == 0 {
                byte_y >> 4
            } else {
                byte_y & 0x0F
            };
            r[i] = Some(b.ry(r[i].take().unwrap(), -(nibble_y as f64) * PI / 8.0));

            let byte_z = data[l * NUM_QUBITS + (NUM_QUBITS + i) / 2];
            let nibble_z = if (NUM_QUBITS + i) % 2 == 0 {
                byte_z >> 4
            } else {
                byte_z & 0x0F
            };
            r[i] = Some(b.rz(r[i].take().unwrap(), (nibble_z as f64) * PI / 8.0));
        }

        for i in 1..r.len() {
            (r[i - 1], r[i]) = match b
                .cnot(r[i - 1].take().unwrap(), r[i].take().unwrap())
                .unwrap()
            {
                (q1, q2) => (Some(q1), Some(q2)),
            };
        }
    }

    let (statevec, _) = b.calculate_state();
    r.iter()
        .map(|q| {
            1.0 - 2.0
                * measure_prob(
                    NUM_QUBITS,
                    1,
                    &[*q.as_ref().unwrap().indices().first().unwrap()],
                    &statevec,
                    None,
                )
        })
        .collect::<Vec<_>>()
}
