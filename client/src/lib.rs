use reqwest::Url;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    account::Account, commitment_config::CommitmentLevel, hash::Hash, pubkey::Pubkey,
    transaction::Transaction,
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;

pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3030";
pub const DEFAULT_RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com/";

pub struct ExecutorClient {
    pub url: Url,
    http_client: reqwest::blocking::Client,
}

pub struct ExecutorClientConfig {
    pub executor_server_url: Url,
    pub rpc_endpoint: Url,
    pub rpc_commitment: CommitmentLevel,
}

impl Default for ExecutorClientConfig {
    fn default() -> Self {
        Self {
            executor_server_url: Url::parse(DEFAULT_SERVER_URL).unwrap(),
            rpc_commitment: CommitmentLevel::Processed,
            rpc_endpoint: Url::parse(DEFAULT_RPC_ENDPOINT).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RpcConfig {
    pub rpc_endpoint: String,
    pub commitment_level: CommitmentLevel,
}

pub type ClientResult<T> = Result<T, reqwest::Error>;

impl Default for ExecutorClient {
    fn default() -> Self {
        ExecutorClient::new()
    }
}

impl ExecutorClient {
    pub fn new() -> Self {
        Self::new_with_config(ExecutorClientConfig::default())
    }

    pub fn new_with_config(config: ExecutorClientConfig) -> Self {
        let executor_client = ExecutorClient {
            url: config.executor_server_url,
            http_client: reqwest::blocking::Client::new(),
        };
        executor_client
            .set_rpc_config(RpcConfig {
                rpc_endpoint: config.rpc_endpoint.to_string(),
                commitment_level: config.rpc_commitment,
            })
            .unwrap();

        executor_client
    }

    pub fn get_latest_blockhash(&self) -> ClientResult<Hash> {
        self.http_client
            .get(self.build_url("/latest_blockhash"))
            .send()?
            .json::<Hash>()
    }

    pub fn advance_blockhash(&self, hash: Option<Hash>) -> ClientResult<Hash> {
        self.http_client
            .post(self.build_url("/advance_blockhash"))
            .json(&hash)
            .send()?
            .json::<Hash>()
    }

    pub fn set_rpc_config(
        &self,
        rpc_config: RpcConfig,
    ) -> ClientResult<reqwest::blocking::Response> {
        self.http_client
            .post(self.build_url("/set_rpc_config"))
            .json(&rpc_config)
            .send()
    }

    pub fn get_rent_exempt_balance(&self, data_length: usize) -> ClientResult<u64> {
        self.http_client
            .get(self.build_url("/rent_exempt_balance"))
            .json(&data_length)
            .send()?
            .json::<u64>()
    }

    pub fn get_account(&self, pubkey: &Pubkey) -> ClientResult<Option<Account>> {
        self.http_client
            .get(self.build_url("/get_account"))
            .json(pubkey)
            .send()?
            .json::<Option<Account>>()
    }

    pub fn get_accounts(&self, pubkeys: &Vec<Pubkey>) -> ClientResult<Vec<Option<Account>>> {
        self.http_client
            .get(self.build_url("/get_accounts"))
            .json(pubkeys)
            .send()?
            .json::<Vec<Option<Account>>>()
    }

    pub fn execute_transaction_batch(
        &self,
        batch: Vec<Transaction>,
    ) -> ClientResult<Vec<EncodedConfirmedTransactionWithStatusMeta>> {
        self.http_client
            .post(self.build_url("/execute_transaction_batch"))
            .json(&batch)
            .send()?
            .json::<Vec<EncodedConfirmedTransactionWithStatusMeta>>()
    }

    fn build_url(&self, path: &str) -> Url {
        let mut url = Url::from_str(self.url.as_str()).unwrap();
        url.set_path(path);

        url
    }
}
