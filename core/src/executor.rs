use crate::{
    programs::{
        self, BPF_LOADER2_PID, BPF_LOADER_UPGRADEABLE_PID, SPL_ASSOCIATED_TOKEN_PID, SPL_MEMO1_PID,
        SPL_TOKEN_PID, SYSTEM_PID, SYSVAR_PID, SYSVAR_RENT_ADDRESS,
    },
    utils::{clone_keypair, random_keypair},
};
use executor_client::DEFAULT_RPC_ENDPOINT;
use itertools::{izip, Itertools};
use solana_bpf_loader_program::{
    solana_bpf_loader_deprecated_program, solana_bpf_loader_program,
    solana_bpf_loader_upgradeable_program,
};
use solana_client::{client_error::reqwest::Url, rpc_client::RpcClient};
use solana_ledger::token_balances;
use solana_runtime::{
    accounts_db::AccountShrinkThreshold,
    accounts_index::AccountSecondaryIndexes,
    bank::{
        Bank, TransactionBalancesSet, TransactionExecutionDetails, TransactionExecutionResult,
        TransactionResults,
    },
    builtins::{Builtin, Builtins},
    genesis_utils,
};
use solana_sdk::{
    account::Account,
    account::AccountSharedData,
    clock::UnixTimestamp,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    feature_set,
    genesis_config::GenesisConfig,
    hash::Hash,
    message::{v0::LoadedAddresses, SanitizedMessage},
    packet,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use solana_transaction_status::{
    ConfirmedTransactionWithStatusMeta, EncodedConfirmedTransactionWithStatusMeta,
    InnerInstructions, TransactionStatusMeta, TransactionTokenBalance, TransactionWithStatusMeta,
    UiTransactionEncoding, VersionedTransactionWithStatusMeta,
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Executor {
    bank: Bank,
    faucet: Keypair,
    rpc_client: RpcClient,
}

impl Executor {
    pub fn builder_with_config(config: ExecutorConfig) -> ExecutorBuilder {
        ExecutorBuilder::new_with_config(config)
    }

    pub fn new_with_config(config: ExecutorConfig) -> Executor {
        Self::builder_with_config(config).build()
    }

    pub fn bank(&self) -> &Bank {
        &self.bank
    }

    pub fn bank_mut(&mut self) -> &mut Bank {
        &mut self.bank
    }

    pub fn payer(&self) -> Keypair {
        clone_keypair(&self.faucet)
    }

    pub fn get_minimum_rent_exempt_balance(&self, data_len: usize) -> u64 {
        self.bank.get_minimum_balance_for_rent_exemption(data_len)
    }

    pub fn get_latest_blockhash(&self) -> Hash {
        self.bank.confirmed_last_blockhash()
    }

    pub fn get_account(&self, pubkey: &Pubkey) -> Option<Account> {
        self.bank().get_account(pubkey).map(From::from)
    }

    pub fn get_accounts(&self, pubkeys: &[Pubkey]) -> Vec<Option<Account>> {
        pubkeys
            .iter()
            .map(|pk| self.bank().get_account(pk).map(From::from))
            .collect_vec()
    }

    pub fn set_rpc_config(&mut self, rpc_endpoint: String, commitment_level: CommitmentLevel) {
        self.rpc_client = RpcClient::new_with_commitment(
            rpc_endpoint,
            CommitmentConfig {
                commitment: commitment_level,
            },
        );
    }

    pub fn advance_blockhash(&self, hash: Option<Hash>) -> Hash {
        let parent_distance = if self.bank.slot() == 0 {
            1
        } else {
            self.bank.slot() - self.bank.parent_slot()
        };

        for _ in 0..parent_distance {
            let last_blockhash = self.bank.last_blockhash();
            let new_hash = match hash {
                Some(new_hash) if new_hash != self.bank.last_blockhash() => new_hash,
                _ => Hash::new_unique(),
            };

            while self.bank.last_blockhash() == last_blockhash {
                self.bank.register_tick(&new_hash)
            }
        }
        self.get_latest_blockhash()
    }

    pub fn execute_transaction_internal(
        &mut self,
        tx: &Transaction,
    ) -> EncodedConfirmedTransactionWithStatusMeta {
        let len = bincode::serialize(&tx).unwrap().len();
        if len > packet::PACKET_DATA_SIZE {
            panic!(
                "tx {:?} of size {} is {} too large",
                tx,
                len,
                len - packet::PACKET_DATA_SIZE
            )
        }
        let txs = vec![tx.clone()];

        let batch = self.bank.prepare_batch_for_tests(txs.clone());
        let mut mint_decimals = HashMap::new();
        let tx_pre_token_balances =
            token_balances::collect_token_balances(&self.bank, &batch, &mut mint_decimals);
        let slot = self.bank.slot();
        let mut timings = Default::default();
        let (
            TransactionResults {
                execution_results, ..
            },
            TransactionBalancesSet {
                pre_balances,
                post_balances,
                ..
            },
        ) = self.bank.load_execute_and_commit_transactions(
            &batch,
            usize::MAX,
            true,
            true,
            true,
            true,
            &mut timings,
            None,
        );

        let tx_post_token_balances =
            token_balances::collect_token_balances(&self.bank, &batch, &mut mint_decimals);
        izip!(
            txs.iter(),
            execution_results.into_iter(),
            pre_balances.into_iter(),
            post_balances.into_iter(),
            tx_pre_token_balances.into_iter(),
            tx_post_token_balances.into_iter(),
        )
        .map(
            |(
                tx,
                execution_result,
                pre_balances,
                post_balances,
                pre_token_balances,
                post_token_balances,
            ): ZippedItem| {
                let fee = self.bank.get_fee_for_message(&SanitizedMessage::try_from(tx.message().clone()).expect("Failed to sanitize transaction"))
                    .expect("Fee calculation must succeed");

                let (status, inner_instructions, log_messages, executed_units) = match execution_result {
                    TransactionExecutionResult::Executed { details: TransactionExecutionDetails { status, inner_instructions, log_messages, executed_units, .. }, .. } =>
                        (status, inner_instructions, log_messages, executed_units),
                    TransactionExecutionResult::NotExecuted(err) => (Err(err), None, None, 0)
                };

                let inner_instructions = inner_instructions.map(|inner_instructions| {
                    inner_instructions
                        .into_iter()
                        .enumerate()
                        .map(|(index, instructions)| InnerInstructions {
                            index: index as u8,
                            instructions,
                        })
                        .filter(|i| !i.instructions.is_empty())
                        .collect()
                });

                let tx_status_meta = TransactionStatusMeta {
                    status,
                    fee,
                    pre_balances,
                    post_balances,
                    pre_token_balances: (pre_token_balances).into(),
                    post_token_balances: (post_token_balances).into(),
                    inner_instructions,
                    log_messages,
                    rewards: None,
                    loaded_addresses: LoadedAddresses {
                        writable: vec![], // TODO
                        readonly: vec![], // TODO
                    },
                    return_data: None,
                    compute_units_consumed: executed_units.into()
                };

                ConfirmedTransactionWithStatusMeta {
                    slot,
                    tx_with_meta: TransactionWithStatusMeta::Complete(VersionedTransactionWithStatusMeta {
                        transaction: VersionedTransaction::from(tx.clone()),
                        meta: tx_status_meta,
                    }),
                    block_time: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            .try_into()
                            .unwrap(),
                    ),
                }
                .encode(UiTransactionEncoding::Binary, None)
                .expect("Failed to encode transaction")
            },
        )
        .next().expect("transaction could not be executed. Enable debug logging to get more information on why")
    }

    pub fn execute_transaction_batch(
        &mut self,
        batch: &[Transaction],
    ) -> Vec<EncodedConfirmedTransactionWithStatusMeta> {
        // Extract account keys from batch
        let account_keys = batch
            .iter()
            .flat_map(|tx| tx.message.account_keys.clone())
            .sorted()
            .dedup()
            .collect_vec();

        // Fetch corresponding accounts from target cluster
        let account_infos = self
            .rpc_client
            .get_multiple_accounts(&account_keys)
            .unwrap()
            .iter()
            .zip(account_keys.iter())
            .filter_map(|(account_info, address)| {
                account_info
                    .as_ref()
                    .map(|account_info| (*address, account_info.clone()))
            })
            .collect_vec();

        // Inspect accounts to find programs, and list program data accounts to fetch
        let program_data_account_keys = account_infos
            .iter()
            .filter_map(|(address, account_info)| match account_info.executable {
                true => Some(
                    Pubkey::find_program_address(&[address.as_ref()], &BPF_LOADER_UPGRADEABLE_PID)
                        .0,
                ),
                false => None,
            })
            .sorted()
            .dedup()
            .collect_vec();

        // Fetch corresponding accounts from target cluster
        let account_infos_2 = self
            .rpc_client
            .get_multiple_accounts(&program_data_account_keys)
            .unwrap()
            .iter()
            .zip(program_data_account_keys.iter())
            .filter_map(|(account_info, address)| {
                account_info
                    .as_ref()
                    .map(|account_info| (*address, account_info.clone()))
            })
            .collect_vec();

        // Load all accounts' data in local bank
        for (address, account_info) in [account_infos, account_infos_2].concat() {
            self.bank_mut().store_account(
                &address,
                &Account {
                    lamports: account_info.lamports,
                    data: account_info.data.to_vec(),
                    executable: account_info.executable,
                    owner: account_info.owner,
                    rent_epoch: account_info.rent_epoch,
                },
            )
        }

        batch
            .iter()
            .map(|tx| self.execute_transaction_internal(tx))
            .collect_vec()
    }
}

type ZippedItem<'a> = (
    &'a Transaction,
    TransactionExecutionResult,
    Vec<u64>,
    Vec<u64>,
    Vec<TransactionTokenBalance>,
    Vec<TransactionTokenBalance>,
);

pub struct ExecutorConfig {
    rpc_endpoint: Option<Url>,
    commitment_level: Option<CommitmentLevel>,
    faucet: Keypair,
    genesis_config: GenesisConfig,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            rpc_endpoint: None,
            commitment_level: None,
            faucet: random_keypair(),
            genesis_config: GenesisConfig::default(),
        }
    }
}

pub struct ExecutorBuilder {
    config: GenesisConfig,
    faucet: Keypair,
    rpc_endpoint: Url,
    commitment_level: CommitmentLevel,
}

impl Default for ExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutorBuilder {
    pub fn new() -> Self {
        let faucet = random_keypair();
        let genesis_config = GenesisConfig::new(
            &[(
                faucet.pubkey(),
                AccountSharedData::new(1u64 << 48, 0, &SYSTEM_PID),
            )],
            &[],
        );

        Self::new_with_config(ExecutorConfig {
            rpc_endpoint: None,
            commitment_level: None,
            genesis_config,
            faucet,
        })
    }

    pub fn new_with_config(mut config: ExecutorConfig) -> Self {
        genesis_utils::activate_all_features(&mut config.genesis_config);
        config
            .genesis_config
            .accounts
            .remove(&feature_set::fix_recent_blockhashes::id());

        let mut builder = ExecutorBuilder {
            faucet: config.faucet,
            config: config.genesis_config,
            rpc_endpoint: config
                .rpc_endpoint
                .unwrap_or_else(|| Url::parse(DEFAULT_RPC_ENDPOINT).unwrap()),
            commitment_level: config
                .commitment_level
                .unwrap_or(CommitmentLevel::Processed),
        };
        builder.add_rent_exempt_account_with_data(
            SPL_ASSOCIATED_TOKEN_PID,
            BPF_LOADER2_PID,
            programs::SPL_ASSOCIATED_TOKEN,
            true,
        );
        builder.add_rent_exempt_account_with_data(
            "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo"
                .parse()
                .unwrap(),
            BPF_LOADER2_PID,
            programs::SPL_MEMO1,
            true,
        );
        builder.add_rent_exempt_account_with_data(
            SPL_MEMO1_PID,
            BPF_LOADER2_PID,
            programs::SPL_MEMO3,
            true,
        );
        builder.add_rent_exempt_account_with_data(
            SPL_TOKEN_PID,
            BPF_LOADER2_PID,
            programs::SPL_TOKEN,
            true,
        );
        builder.add_account_with_lamports(SYSVAR_RENT_ADDRESS, SYSVAR_PID, 1);

        builder.set_creation_time(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        );

        builder
    }

    fn set_creation_time(&mut self, unix_timestamp: UnixTimestamp) -> &mut Self {
        self.config.creation_time = unix_timestamp as UnixTimestamp;
        self
    }

    pub fn add_account(&mut self, pubkey: Pubkey, account: Account) -> &mut Self {
        self.config.add_account(pubkey, account.into());
        self
    }

    pub fn add_program<P: AsRef<Path>>(&mut self, pubkey: Pubkey, path: P) -> &mut Self {
        self.add_rent_exempt_account_with_data(
            pubkey,
            BPF_LOADER2_PID,
            &std::fs::read(path).unwrap(),
            true,
        );
        self
    }

    pub fn add_rent_exempt_account_with_data(
        &mut self,
        pubkey: Pubkey,
        owner: Pubkey,
        data: &[u8],
        executable: bool,
    ) -> &mut Self {
        self.add_account(
            pubkey,
            Account {
                lamports: self.config.rent.minimum_balance(data.len()),
                data: data.to_vec(),
                executable,
                owner,
                rent_epoch: 0,
            },
        )
    }

    pub fn add_account_with_lamports(
        &mut self,
        pubkey: Pubkey,
        owner: Pubkey,
        lamports: u64,
    ) -> &mut Self {
        self.add_account(
            pubkey,
            Account {
                lamports,
                data: vec![],
                executable: false,
                owner,
                rent_epoch: 0,
            },
        )
    }

    /// Finalizes the environment.
    pub fn build(&mut self) -> Executor {
        let tmpdir = Path::new("/tmp/");

        let bank = Bank::new_with_paths(
            &self.config,
            // runtime_config,
            vec![tmpdir.to_path_buf()],
            None,
            Some(&Builtins {
                genesis_builtins: [
                    solana_bpf_loader_upgradeable_program!(),
                    solana_bpf_loader_program!(),
                    solana_bpf_loader_deprecated_program!(),
                ]
                .iter()
                .map(|p| Builtin::new(&p.0, p.1, p.2))
                .collect(),
                feature_transitions: vec![],
            }),
            AccountSecondaryIndexes {
                keys: None,
                indexes: HashSet::new(),
            },
            false,
            AccountShrinkThreshold::default(),
            false,
            None,
            None,
        );

        let executor = Executor {
            bank,
            faucet: clone_keypair(&self.faucet),
            rpc_client: RpcClient::new_with_commitment(
                self.rpc_endpoint.clone(),
                CommitmentConfig {
                    commitment: self.commitment_level,
                },
            ),
        };
        executor.advance_blockhash(None);

        executor
    }
}
