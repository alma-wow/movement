use crate::common_args::MovementArgs;
use crate::node::partial::MovementPartialNode;
use anyhow::Context;
use clap::Parser;
use maptos_dof_execution::DynOptFinExecutor;
use maptos_opt_executor::executor::TxExecutionResult;
use mcr_settlement_client::{McrSettlementClient, McrSettlementClientOperations};
use tokio::sync::mpsc::unbounded_channel;
use tracing::info;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Gets the block commitment that this validator (or another provided validator) has settled at a given height."
)]
pub struct Commitment {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl Commitment {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		info!("Forcing commitment");
		let config = self.movement_args.config().await?;
		info!("Loaded config {:?}", config);
		let settlement_client = McrSettlementClient::build_with_config(&config.mcr)
			.await
			.context("Failed to build MCR settlement client with config")?;
		info!("Built settlement client");

		//No Tx are processed so no need to manage the receiver.
		let (mempool_tx_exec_result_sender, _mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();

		let executor =
			MovementPartialNode::try_executor_from_config(config, mempool_tx_exec_result_sender)
				.await
				.context("Failed to create the executor")?;

		let height = match self.height {
			Some(height) => height,
			None => executor.get_block_head_height()?,
		};

		executor.revert_block_head_to(height).await?;
		let commitment = settlement_client.get_posted_commitment_at_height(height).await?;

		// Use println as this is standard (non-logging output)
		match commitment {
			Some(commitment) => println!("{}", commitment),
			None => println!("No commitment found at height {}", height),
		}

		Ok(())
	}
}
