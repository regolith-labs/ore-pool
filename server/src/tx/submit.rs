use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use crate::error::Error;

const JITO_TIP_AMOUNT: u64 = 2_000;
pub const JITO_TIP_ADDRESSES: [Pubkey; 8] = [
    solana_sdk::pubkey!("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5"),
    solana_sdk::pubkey!("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe"),
    solana_sdk::pubkey!("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"),
    solana_sdk::pubkey!("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49"),
    solana_sdk::pubkey!("DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh"),
    solana_sdk::pubkey!("ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt"),
    solana_sdk::pubkey!("DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL"),
    solana_sdk::pubkey!("3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT"),
];

pub async fn submit_and_confirm_instructions(
    signer: &Keypair,
    rpc_client: &RpcClient,
    jito_client: &RpcClient,
    ixs: &[Instruction],
    cu_limit: u32,
    cu_price: u64,
) -> Result<Signature, Error> {
    let max_retries = 5;
    let mut retries = 0;
    while retries < max_retries {
        let sig =
            submit_instructions(signer, rpc_client, jito_client, ixs, cu_limit, cu_price).await;
        match sig {
            Ok(sig) => match confirm_transaction(rpc_client, &sig).await {
                Ok(()) => return Ok(sig),
                Err(err) => {
                    log::error!("failed to confirm signature: {:?}", err);
                    retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            },
            Err(err) => {
                log::error!("failed to submit transaction: {:?}", err);
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
    Err(Error::Internal(
        "failed to land transaction with confirmation".to_string(),
    ))
}

pub async fn submit_instructions(
    signer: &Keypair,
    rpc_client: &RpcClient,
    jito_client: &RpcClient,
    ixs: &[Instruction],
    cu_limit: u32,
    cu_price: u64,
) -> Result<Signature, Error> {
    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(cu_limit);
    let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(cu_price);
    let tip_ix = tip_ix(&signer.pubkey());
    let final_ixs = &[cu_limit_ix, cu_price_ix];
    let final_ixs = [final_ixs, ixs, &[tip_ix]].concat();
    let hash = rpc_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(final_ixs.as_slice(), Some(&signer.pubkey()));
    tx.sign(&[signer], hash);
    let sim = rpc_client.simulate_transaction(&tx).await?;
    log::info!("sim: {:?}", sim);
    jito_client.send_transaction(&tx).await.map_err(From::from)
}

pub async fn submit_and_confirm_transaction(
    rpc_client: &RpcClient,
    tx: &Transaction,
) -> Result<Signature, Error> {
    let max_retries = 5;
    let mut retries = 0;
    while retries < max_retries {
        let sig = rpc_client.send_transaction(tx).await;
        match sig {
            Ok(sig) => match confirm_transaction(rpc_client, &sig).await {
                Ok(()) => return Ok(sig),
                Err(err) => {
                    log::info!("{:?}", err);
                    retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            },
            Err(err) => {
                log::info!("{:?}", err);
                retries += 1;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
    Err(Error::Internal(
        "failed to land transaction with confirmation".to_string(),
    ))
}

async fn confirm_transaction(rpc_client: &RpcClient, sig: &Signature) -> Result<(), Error> {
    // Confirm the transaction with retries
    let max_retries = 10;
    let mut retries = 0;
    while retries < max_retries {
        if let Ok(confirmed) = rpc_client
            .confirm_transaction_with_commitment(sig, CommitmentConfig::confirmed())
            .await
        {
            if confirmed.value {
                break;
            }
        }
        retries += 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    if retries == max_retries {
        return Err(Error::Internal("could not confirm transaction".to_string()));
    }
    Ok(())
}

fn tip_ix(signer: &Pubkey) -> Instruction {
    let address = get_jito_tip_address();
    solana_sdk::system_instruction::transfer(signer, &address, JITO_TIP_AMOUNT)
}

fn get_jito_tip_address() -> Pubkey {
    let random_index = rand::random::<usize>() % JITO_TIP_ADDRESSES.len();
    JITO_TIP_ADDRESSES[random_index]
}
