use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use crate::error::Error;

pub async fn submit_and_confirm(
    signer: &Keypair,
    rpc_client: &RpcClient,
    ixs: &[Instruction],
    cu_limit: u32,
    cu_price: u64,
) -> Result<Signature, Error> {
    let max_retries = 5;
    let mut retries = 0;
    while retries < max_retries {
        let sig = submit(signer, rpc_client, ixs, cu_limit, cu_price).await?;
        match confirm_transaction(rpc_client, &sig).await {
            Ok(()) => return Ok(sig),
            Err(err) => {
                log::info!("{:?}", err);
                retries += 1;
            }
        }
    }
    Err(Error::Internal(
        "failed to land transaction with confirmation".to_string(),
    ))
}

pub async fn submit(
    signer: &Keypair,
    rpc_client: &RpcClient,
    ixs: &[Instruction],
    cu_limit: u32,
    cu_price: u64,
) -> Result<Signature, Error> {
    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(cu_limit);
    let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(cu_price);
    let final_ixs = &[cu_limit_ix, cu_price_ix];
    let final_ixs = [final_ixs, ixs].concat();
    let hash = rpc_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(final_ixs.as_slice(), Some(&signer.pubkey()));
    tx.sign(&[signer], hash);
    rpc_client.send_transaction(&tx).await.map_err(From::from)
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
