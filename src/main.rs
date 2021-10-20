use ethers::{
    abi::Abi,
    contract::{Contract, EthAbiType},
    prelude::SignerMiddleware,
    providers::{Http, Provider},
    signers::{HDPath, Ledger, Signer},
    types::{transaction::eip712::Eip712, Address},
};
use ethers_derive_eip712::*;
use std::convert::TryFrom;

const RPC_URL: &str = "https://eth-rinkeby.alchemyapi.io/v2/JWI1qhmWEYMLwfWjnKGW2WTpHh_c69Fv";
const REWARD_CONTRACT: &str = "0xb92557a73f997c74ef6bc2acd63501a281b4d888";
const REWARD_ABI: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/abis/RewardV1.json"));

#[derive(Debug, Clone, Eip712, EthAbiType, serde::Serialize, serde::Deserialize)]
#[eip712(
    name = "Radicle",
    version = "1",
    chain_id = 4,
    verifying_contract = "0xb92557a73f997c74ef6bc2acd63501a281b4d888"
)]
pub struct Puzzle {
    org: Address,
    contributor: Address,
    commit: [u8; 32],
    project: [u8; 32],
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider =
        Provider::<Http>::try_from(RPC_URL).expect("could not instantiate HTTP Provider");

    let puzzle = Puzzle {
        org: "0xf696fa356b9d25190289ce766ee17e42f6a058c9"
            .parse::<Address>()
            .unwrap(),
        contributor: "0x4b3c2ca982504af250fb68315d9c016688028edd"
            .parse::<Address>()
            .unwrap(),
        commit: [
            145, 49, 203, 35, 247, 97, 19, 44, 180, 217, 97, 183, 48, 74, 156, 79, 170, 241, 225,
            30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        project: [
            224, 136, 160, 7, 151, 3, 174, 108, 245, 139, 196, 1, 206, 186, 199, 158, 0, 155, 151,
            78, 23, 105, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    let prompt = format!("{} Password: ", "??");
    let password = rpassword::prompt_password_stdout(&prompt)?;
    let wallet = ethers::signers::LocalWallet::decrypt_keystore("./contributor.json", password)?;
    let signer = wallet.with_chain_id(4u64);

    let signer = SignerMiddleware::new(provider, signer);

    let v: u8 = 27;
    let r: [u8; 32] = [
        238, 72, 117, 52, 176, 163, 106, 36, 59, 201, 59, 7, 204, 169, 116, 128, 44, 9, 5, 105, 96,
        51, 140, 140, 179, 50, 145, 7, 223, 224, 35, 0,
    ];
    let s: [u8; 32] = [
        110, 94, 210, 160, 151, 39, 143, 84, 149, 36, 48, 120, 31, 90, 228, 76, 224, 134, 206, 0,
        184, 218, 176, 112, 17, 235, 125, 114, 200, 13, 197, 28,
    ];
    let abi: Abi = serde_json::from_str(REWARD_ABI)?;
    let contract = Contract::new(REWARD_CONTRACT.parse::<Address>().unwrap(), abi, signer);
    let call = contract
        .method::<_, bool>("claimRewardEOA", (puzzle, v, r, s))?;

    // let estimate = call.estimate_gas().await?;

    // The gas estimation seems to not work, since the msg. sender seems to be different.
    // println!("gas estimate: {}", estimate);

    // Sometimes not defining a fixed gas limit works and sometimes it seems to run out of gas..
    // Here different transactions without gas specifications that ran out of gas
    // https://rinkeby.etherscan.io/address/0xb92557a73f997c74ef6bc2acd63501a281b4d888
    // let call = call.gas(500000);

    let result = loop {
        let pending = call.send().await?;
        let tx_hash = *pending;

        if let Some(result) = pending.await? {
            break result;
        } else {
            println!("Transaction {} dropped, retrying..", tx_hash);
        }
    };

    println!(
        "Reward successfully minted in block #{} ({})",
        result.block_number.unwrap(),
        result.block_hash.unwrap(),
    );

    Ok(())
}
