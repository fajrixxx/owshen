use crate::fp::Fp;

use ff::PrimeField;

use num_bigint::BigUint;

use ethers::prelude::*;

use ethers::abi::ethabi::ethereum_types::FromStrRadixErr;
use eyre::Result;

use std::process::Command;

#[derive(Clone, Debug)]
pub struct Proof {
    a: [U256; 2],
    b: [[U256; 2]; 2],
    c: [U256; 2],
    public: Vec<U256>,
}

use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub fn prove<P: AsRef<Path>>(params: P, a: Fp, b: Fp) -> Result<Proof> {
    let mut inputs_file = NamedTempFile::new()?;
    write!(
        inputs_file,
        "{{ \"a\": {:?}, \"b\": {:?} }}",
        BigUint::from_bytes_le(a.to_repr().as_ref()),
        BigUint::from_bytes_le(b.to_repr().as_ref())
    )?;

    let witness_file = NamedTempFile::new()?;
    let wtns_gen_output = Command::new("contracts/circuits/coin_withdraw_cpp/coin_withdraw")
        .arg(inputs_file.path())
        .arg(witness_file.path())
        .output()?;

    assert_eq!(wtns_gen_output.stdout.len(), 0);
    assert_eq!(wtns_gen_output.stderr.len(), 0);

    let proof_file = NamedTempFile::new()?;
    let pub_inp_file = NamedTempFile::new()?;
    let proof_gen_output = Command::new("snarkjs")
        .arg("groth16")
        .arg("prove")
        .arg(params.as_ref().as_os_str())
        .arg(witness_file.path())
        .arg(proof_file.path())
        .arg(pub_inp_file.path())
        .output()?;

    assert_eq!(proof_gen_output.stdout.len(), 0);
    assert_eq!(proof_gen_output.stderr.len(), 0);

    let generatecall_output = Command::new("snarkjs")
        .arg("generatecall")
        .arg(pub_inp_file.path())
        .arg(proof_file.path())
        .output()?;
    let mut calldata = std::str::from_utf8(&generatecall_output.stdout)?.to_string();
    calldata = calldata
        .replace("\"", "")
        .replace("[", "")
        .replace("]", "")
        .replace(" ", "")
        .replace("\n", "");
    let data = calldata
        .split(",")
        .map(|k| U256::from_str_radix(k, 16))
        .collect::<Result<Vec<U256>, FromStrRadixErr>>()?;

    let proof = Proof {
        a: data[0..2].try_into()?,
        b: [data[2..4].try_into()?, data[4..6].try_into()?],
        c: data[6..8].try_into()?,
        public: data[8..].to_vec(),
    };

    Ok(proof)
}
