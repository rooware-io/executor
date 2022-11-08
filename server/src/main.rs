use executor_core::executor::{Executor, ExecutorConfig};
pub use solana_client::client_error::reqwest::Url;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub struct ContextRaw {
    pub executor: Executor,
}

impl ContextRaw {
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            executor: Executor::new_with_config(config),
        }
    }
}

pub type Context = Arc<Mutex<ContextRaw>>;

#[tokio::main]
pub async fn main() {
    let context = Arc::new(Mutex::new(ContextRaw::new(ExecutorConfig::default())));

    let api = filters::api(context);
    let routes = api.with(warp::log("api"));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

mod filters {
    use super::{handlers, Context};
    use warp::Filter;

    // Routes aggregation
    pub fn api(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        latest_blockhash(context.clone())
            .or(advance_blockhash(context.clone()))
            .or(set_rpc_config(context.clone()))
            .or(rent_exempt_balance(context.clone()))
            .or(get_account(context.clone()))
            .or(get_accounts(context.clone()))
            .or(execute_transaction_batch(context))
    }

    // Route definitions
    pub fn latest_blockhash(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("latest_blockhash")
            .and(warp::get())
            .and(with_context(context))
            .and_then(handlers::latest_blockhash)
    }

    pub fn advance_blockhash(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("advance_blockhash")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_context(context))
            .and_then(handlers::advance_blockhash)
    }

    pub fn set_rpc_config(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("set_rpc_config")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_context(context))
            .and_then(handlers::set_rpc_config)
    }

    pub fn rent_exempt_balance(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("rent_exempt_balance")
            .and(warp::get())
            .and(warp::body::json())
            .and(with_context(context))
            .and_then(handlers::rent_exempt_balance)
    }

    pub fn get_account(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("get_account")
            .and(warp::get())
            .and(warp::body::json())
            .and(with_context(context))
            .and_then(handlers::get_account)
    }

    pub fn get_accounts(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("get_accounts")
            .and(warp::get())
            .and(warp::body::json())
            .and(with_context(context))
            .and_then(handlers::get_accounts)
    }

    pub fn execute_transaction_batch(
        context: Context,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("execute_transaction_batch")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_context(context))
            .and_then(handlers::execute_transaction_batch)
    }

    // Helpers
    fn with_context(
        context: Context,
    ) -> impl Filter<Extract = (Context,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || context.clone())
    }
}

mod handlers {
    use super::Context;
    use executor_client::RpcConfig;
    use solana_program::{hash::Hash, pubkey::Pubkey};
    use solana_sdk::transaction::Transaction;
    use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
    use std::convert::Infallible;
    use warp::hyper::StatusCode;

    pub async fn latest_blockhash(context: Context) -> Result<impl warp::Reply, Infallible> {
        let context = context.lock().await;
        let latest_blockhash = context.executor.get_latest_blockhash();
        Ok(warp::reply::json(&latest_blockhash))
    }

    pub async fn advance_blockhash(
        hash: Option<Hash>,
        context: Context,
    ) -> Result<impl warp::Reply, Infallible> {
        let context = context.lock().await;
        let latest_blockhash = context.executor.advance_blockhash(hash);
        Ok(warp::reply::json(&latest_blockhash))
    }

    pub async fn set_rpc_config(
        rpc_config: RpcConfig,
        context: Context,
    ) -> Result<impl warp::Reply, Infallible> {
        let mut context = context.lock().await;
        context
            .executor
            .set_rpc_config(rpc_config.rpc_endpoint, rpc_config.commitment_level);
        Ok(StatusCode::OK)
    }

    pub async fn rent_exempt_balance(
        data_length: usize,
        context: Context,
    ) -> Result<impl warp::Reply, Infallible> {
        let context = context.lock().await;
        let rent_exempt_balance = context
            .executor
            .get_minimum_rent_exempt_balance(data_length);
        Ok(warp::reply::json(&rent_exempt_balance))
    }

    pub async fn get_account(
        pubkey: Pubkey,
        context: Context,
    ) -> Result<impl warp::Reply, Infallible> {
        let context = context.lock().await;
        let maybe_account = context.executor.get_account(&pubkey);
        Ok(warp::reply::json(&maybe_account))
    }

    pub async fn get_accounts(
        pubkeys: Vec<Pubkey>,
        context: Context,
    ) -> Result<impl warp::Reply, Infallible> {
        let context = context.lock().await;
        let maybe_accounts = context.executor.get_accounts(&pubkeys);
        Ok(warp::reply::json(&maybe_accounts))
    }

    pub async fn execute_transaction_batch(
        batch: Vec<Transaction>,
        context: Context,
    ) -> Result<impl warp::Reply, Infallible> {
        let mut context = context.lock().await;
        let simulation_results: Vec<EncodedConfirmedTransactionWithStatusMeta> =
            context.executor.execute_transaction_batch(&batch);
        Ok(warp::reply::json(&simulation_results))
    }
}
