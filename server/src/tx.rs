use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use crate::error::Error;

pub async fn submit(
    signer: &Keypair,
    rpc_client: &RpcClient,
    ixs: Vec<Instruction>,
    cu_limit: u32,
    cu_price: u64,
) -> Result<Signature, Error> {
    let cu_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(cu_limit);
    let cu_price_ix = ComputeBudgetInstruction::set_compute_unit_price(cu_price);
    let mut final_ixs = vec![cu_limit_ix, cu_price_ix];
    let ixs = ixs.into_iter();
    for ix in ixs {
        final_ixs.push(ix);
    }
    let hash = rpc_client.get_latest_blockhash().await?;
    let mut tx = Transaction::new_with_payer(final_ixs.as_slice(), Some(&signer.pubkey()));
    tx.sign(&[signer], hash);
    rpc_client.send_transaction(&tx).await.map_err(From::from)
}
