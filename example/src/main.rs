executor_client_gen::generate_client!();

use executor_client::{ExecutorClient, ExecutorClientConfig};
use reqwest::Url;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

fn main() {
    let client = ExecutorClient::new();
    let payer = Pubkey::new_unique();
    let latest_blockhash = client.get_latest_blockhash().unwrap();

    let mut transactions = vec![];
    for i in 0..2 {
        let account_size = 1000 + i;
        let account_kp = Keypair::new();
        let rent_exempt_balance = client.get_rent_exempt_balance(account_size).unwrap();
        let mut transaction = Transaction::new_with_payer(
            &vec![system_instruction::create_account(
                &payer,
                &account_kp.pubkey(),
                rent_exempt_balance,
                account_size as u64,
                &system_program::ID,
            )],
            Some(&payer),
        );
        transaction.partial_sign(&[&account_kp], latest_blockhash);
        transactions.push(transaction)
    }

    let results: Vec<EncodedConfirmedTransactionWithStatusMeta> =
        client.execute_transaction_batch(transactions).unwrap();

    for result in results {
        let meta = result.transaction.meta.unwrap();
        if let Some(error) = meta.err {
            println!("Error: {:#?}", error);
            let logs: Option<Vec<String>> = meta.log_messages.into();
            if let Some(logs) = logs {
                println!("Logs:");
                for line in logs {
                    println!("{:#?}", line);
                }
            }
            continue;
        }

        println!("Success");
        println!("{:#?}", &meta.pre_balances);
        println!("{:#?}", &meta.post_balances);
        let logs: Option<Vec<String>> = meta.log_messages.into();
        if let Some(logs) = logs {
            for line in logs {
                println!("{:#?}", line);
            }
        }
    }
}
