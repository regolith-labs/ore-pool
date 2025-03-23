use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use helius::types::Cluster;
use ore_api::state::Proof;
use ore_pool_api::state::{Member, Migration, Pool};
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::address_lookup_table::state::AddressLookupTable;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use solana_sdk::{signature::Keypair, signer::EncodableKey};
use steel::{sysvar, AccountDeserialize, Clock, Discriminator, Instruction};

use crate::error::Error::InvalidHeliusCluster;

pub struct Client {
    pub rpc: solana_client::nonblocking::rpc_client::RpcClient,
    pub keypair: Arc<Keypair>,
}

impl Client {
    pub fn new() -> Result<Self> {
        // let helius_api_key = helius_api_key()?;
        // let helius_cluster = helius_cluster()?;
        let keypair = keypair()?;
        // let rpc = helius::Helius::new_with_async_solana(helius_api_key.as_str(), helius_cluster)?;
        let rpc = RpcClient::new(
            "https://mainnet.helius-rpc.com/?api-key=3e5756b4-fdcb-4a95-883c-8d6603611d1a"
                .to_string(),
        );
        let client = Self {
            rpc,
            keypair: Arc::new(keypair),
        };
        Ok(client)
    }

    pub async fn send_transaction(&self, ixs: &[Instruction]) -> Result<Signature> {
        // let signer = self.keypair.as_ref().secret().to_bytes();
        // let tx = CreateSmartTransactionSeedConfig::new(ixs.to_vec(), vec![signer]);
        let tx = self.create_transaction(ixs).await?;
        let sig = self.rpc.send_transaction(&tx).await?;
        Ok(sig)
    }

    /// returns base58 encoded transaction string
    async fn create_transaction(&self, ixs: &[Instruction]) -> Result<VersionedTransaction> {
        // build transaction
        let hash = self.rpc.get_latest_blockhash().await?;
        let message =
            solana_sdk::message::v0::Message::try_compile(&self.keypair.pubkey(), ixs, &[], hash)?;
        let tx = solana_sdk::transaction::VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(message),
            &[self.keypair.as_ref()],
        )?;
        Ok(tx)
    }
}

#[async_trait]
pub trait AsyncClient {
    // fn get_async_client(&self) -> Result<Arc<RpcClient>>;
    // async fn get_boost(&self, boost: &Pubkey) -> Result<Boost>;
    async fn get_migration(&self, migration: &Pubkey) -> Result<Migration>;
    async fn get_pools(&self) -> Result<Vec<(Pubkey, Pool)>>;
    async fn get_pool(&self, pool: &Pubkey) -> Result<Pool>;
    async fn get_pool_members(&self, pool: &Pubkey) -> Result<Vec<(Pubkey, Member)>>;
    async fn get_proof(&self, address: &Pubkey) -> Result<Proof>;
    // async fn get_boost_stake_accounts(&self, boost: &Pubkey) -> Result<Vec<(Pubkey, Stake)>>;
    // async fn get_boosts_v1(&self) -> Result<Vec<ore_boost_api_v1::state::Boost>>;
    // async fn get_boost_v1_stake_accounts(
    //     &self,
    //     boost: &Pubkey,
    // ) -> Result<Vec<(Pubkey, ore_boost_api_v1::state::Stake)>>;
    // async fn get_stake(&self, stake: &Pubkey) -> Result<ore_boost_api::state::Stake>;
    // async fn get_stake_v1(&self, stake: &Pubkey) -> Result<ore_boost_api_v1::state::Stake>;
    async fn get_clock(&self) -> Result<Clock>;
    async fn get_lookup_table(&self, lut: &Pubkey) -> Result<AddressLookupTableAccount>;
    async fn get_lookup_tables(&self, luts: &[Pubkey]) -> Result<Vec<AddressLookupTableAccount>>;
}

#[async_trait]
impl AsyncClient for solana_client::nonblocking::rpc_client::RpcClient {
    // fn get_async_client(&self) -> Result<Arc<RpcClient>> {
    //     let res = match &self {
    //         Some(rpc) => {
    //             let rpc = Arc::clone(rpc);
    //             Ok(rpc)
    //         }
    //         None => Err(MissingHeliusSolanaAsyncClient),
    //     };
    //     res.map_err(From::from)
    // }
    // async fn get_boost(&self, boost: &Pubkey) -> Result<Boost> {
    //     let data = self.get_account_data(boost).await?;
    //     let boost = Boost::try_from_bytes(data.as_slice())?;
    //     Ok(*boost)
    // }
    // async fn get_boosts(&self) -> Result<Vec<Boost>> {
    //     let accounts = get_program_accounts::<Boost>(self, &ore_boost_api::ID, vec![]).await?;
    //     let accounts = accounts.into_iter().map(|(_, boost)| boost).collect();
    //     Ok(accounts)
    // }
    async fn get_migration(&self, migration: &Pubkey) -> Result<Migration> {
        let data = self.get_account_data(migration).await?;
        let migration = Migration::try_from_bytes(data.as_slice())?;
        Ok(*migration)
    }
    async fn get_proof(&self, address: &Pubkey) -> Result<Proof> {
        let data = self.get_account_data(address).await?;
        let proof = Proof::try_from_bytes(data.as_slice())?;
        Ok(*proof)
    }
    async fn get_pool(&self, pool: &Pubkey) -> Result<Pool> {
        let data = self.get_account_data(pool).await?;
        let pool = Pool::try_from_bytes(data.as_slice())?;
        Ok(*pool)
    }
    async fn get_pools(&self) -> Result<Vec<(Pubkey, Pool)>> {
        get_program_accounts::<Pool>(self, &ore_pool_api::ID, vec![]).await
    }
    async fn get_pool_members(&self, pool: &Pubkey) -> Result<Vec<(Pubkey, Member)>> {
        let filter = RpcFilterType::Memcmp(Memcmp::new_raw_bytes(16, pool.to_bytes().to_vec()));
        let accounts =
            get_program_accounts::<Member>(self, &ore_pool_api::ID, vec![filter]).await?;
        let accounts = accounts
            .into_iter()
            .filter(|(_, member)| member.pool.eq(pool))
            .collect();
        Ok(accounts)
    }
    // async fn get_boosts_v1(&self) -> Result<Vec<ore_boost_api_v1::state::Boost>> {
    //     let accounts = get_program_accounts::<ore_boost_api_v1::state::Boost>(
    //         self,
    //         &ore_boost_api_v1::ID,
    //         vec![],
    //     )
    //     .await?;
    //     let accounts = accounts.into_iter().map(|(_, boost)| boost).collect();
    //     Ok(accounts)
    // }
    // async fn get_boost_v1_stake_accounts(
    //     &self,
    //     boost: &Pubkey,
    // ) -> Result<Vec<(Pubkey, ore_boost_api_v1::state::Stake)>> {
    //     let accounts = get_program_accounts::<ore_boost_api_v1::state::Stake>(
    //         self,
    //         &ore_boost_api_v1::ID,
    //         vec![],
    //     )
    //     .await?;
    //     let accounts = accounts
    //         .into_iter()
    //         .filter(|(_, stake)| stake.boost.eq(boost))
    //         .collect();
    //     Ok(accounts)
    // }
    // async fn get_stake(&self, stake: &Pubkey) -> Result<ore_boost_api::state::Stake> {
    //     let data = self.get_account_data(stake).await?;
    //     let stake = ore_boost_api::state::Stake::try_from_bytes(data.as_slice())?;
    //     Ok(*stake)
    // }
    // async fn get_stake_v1(&self, stake: &Pubkey) -> Result<ore_boost_api_v1::state::Stake> {
    //     let data = self.get_account_data(stake).await?;
    //     let stake = ore_boost_api_v1::state::Stake::try_from_bytes(data.as_slice())?;
    //     Ok(*stake)
    // }
    async fn get_clock(&self) -> Result<Clock> {
        let data = self.get_account_data(&sysvar::clock::ID).await?;
        let clock = bincode::deserialize::<Clock>(data.as_slice())?;
        Ok(clock)
    }
    async fn get_lookup_table(&self, lut: &Pubkey) -> Result<AddressLookupTableAccount> {
        let rpc = self;
        let data = rpc.get_account_data(lut).await?;
        let account = AddressLookupTable::deserialize(data.as_slice())?;
        let account = AddressLookupTableAccount {
            key: *lut,
            addresses: account.addresses.to_vec(),
        };
        Ok(account)
    }
    async fn get_lookup_tables(&self, luts: &[Pubkey]) -> Result<Vec<AddressLookupTableAccount>> {
        // need address for each account so fetch sequentially
        // get multiple accounts does not return the respective pubkeys
        let mut accounts = vec![];
        for lut in luts {
            let account = self.get_lookup_table(lut).await?;
            accounts.push(account);
        }
        Ok(accounts)
    }
}

async fn get_program_accounts<T>(
    client: &RpcClient,
    program_id: &Pubkey,
    filters: Vec<RpcFilterType>,
) -> Result<Vec<(Pubkey, T)>>
where
    T: AccountDeserialize + Discriminator + Copy,
{
    let mut all_filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
        0,
        T::discriminator().to_le_bytes().to_vec(),
    ))];
    all_filters.extend(filters);
    let result = client
        .get_program_accounts_with_config(
            program_id,
            RpcProgramAccountsConfig {
                // filters: Some(all_filters),
                filters: None,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await?;
    let accounts = result
        .into_iter()
        .flat_map(|(pubkey, account)| {
            let account = T::try_from_bytes(&account.data)?;
            Ok::<_, anyhow::Error>((pubkey, *account))
        })
        .collect();
    Ok(accounts)
}

fn helius_api_key() -> Result<String> {
    let key = std::env::var("HELIUS_API_KEY")?;
    Ok(key)
}

fn helius_cluster() -> Result<Cluster> {
    let cluster_str = std::env::var("HELIUS_CLUSTER")?;
    let res = match cluster_str.as_str() {
        "mainnet" => Ok(Cluster::MainnetBeta),
        "mainnet-staked" => Ok(Cluster::StakedMainnetBeta),
        "devnet" => Ok(Cluster::Devnet),
        _ => Err(InvalidHeliusCluster),
    };
    res.map_err(From::from)
}

fn keypair() -> Result<Keypair> {
    let keypair_path = std::env::var("KEYPAIR_PATH")?;
    let keypair =
        Keypair::read_from_file(keypair_path).map_err(|err| anyhow::anyhow!(err.to_string()))?;
    Ok(keypair)
}
