use crate::transaction::Transaction;
use aptos_types::state_proof::StateProof;
use serde::{Deserialize, Serialize};
use std::collections::btree_set;
use std::collections::BTreeSet;
use std::fmt;
use std::result::Result;

pub type Transactions<'a> = btree_set::Iter<'a, Transaction>;

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
	#[error("Error Block Full")]
	BlockFull,
}

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Id([u8; Id::SIZE]);

impl Id {
	pub const SIZE: usize = 32;

	pub fn new(data: [u8; Self::SIZE]) -> Self {
		Self(data)
	}

	pub fn as_bytes(&self) -> &[u8; Self::SIZE] {
		&self.0
	}

	pub fn test() -> Self {
		Self([0; Self::SIZE])
	}

	pub fn to_vec(&self) -> Vec<u8> {
		self.0.into()
	}

	pub fn genesis_block() -> Self {
		Self([0; Self::SIZE])
	}
}

impl AsRef<[u8]> for Id {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for byte in &self.0 {
			write!(f, "{:02x}", byte)?;
		}
		Ok(())
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockMetadata {
	#[default]
	BlockMetadata,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Block {
	metadata: BlockMetadata,
	parent: Id,
	transactions: BTreeSet<Transaction>,
	id: Id,
	timestamp: u64,
}

impl Block {
	pub fn new(metadata: BlockMetadata, parent: Id, transactions: BTreeSet<Transaction>) -> Self {
		let timestamp = chrono::Utc::now().timestamp_micros() as u64;
		let id = Self::generate_id_with_block_data(parent, timestamp, &transactions);
		Self { metadata, parent, transactions, id, timestamp }
	}

	fn generate_id_with_block_data(
		parent: Id,
		timestamp: u64,
		transactions: &BTreeSet<Transaction>,
	) -> Id {
		let mut hasher = blake3::Hasher::new();
		hasher.update(parent.as_bytes());
		hasher.update(&timestamp.to_le_bytes());
		for transaction in transactions {
			hasher.update(&transaction.id().as_ref());
		}
		let id = Id(hasher.finalize().into());
		id
	}

	pub fn into_parts(self) -> (BlockMetadata, Id, BTreeSet<Transaction>, Id) {
		(self.metadata, self.parent, self.transactions, self.id)
	}

	pub fn id(&self) -> Id {
		self.id
	}

	pub fn parent(&self) -> Id {
		self.parent
	}

	pub fn transactions(&self) -> Transactions {
		self.transactions.iter()
	}

	pub fn timestamp(&self) -> u64 {
		self.timestamp
	}

	pub fn metadata(&self) -> &BlockMetadata {
		&self.metadata
	}

	pub fn test() -> Self {
		Self::new(
			BlockMetadata::BlockMetadata,
			Id::test(),
			BTreeSet::from_iter(vec![Transaction::test()]),
		)
	}

	pub fn add_transaction(&mut self, transaction: Transaction) -> Result<(), BlockError> {
		self.transactions.insert(transaction);
		Ok(())
	}

	pub fn collapse(blocks: Vec<Block>) -> Block {
		let mut transactions = BTreeSet::new();
		let parent = if let Some(first_block) = blocks.first() {
			first_block.parent
		} else {
			Id::genesis_block()
		};

		for block in blocks {
			for transaction in block.transactions {
				transactions.insert(transaction);
			}
		}

		Block::new(BlockMetadata::BlockMetadata, parent, transactions)
	}

	pub fn verify_id(&self) -> bool {
		let block_id =
			Self::generate_id_with_block_data(self.parent, self.timestamp, &self.transactions);
		block_id == self.id
	}
}

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Commitment([u8; Id::SIZE]);

impl Commitment {
	pub fn new(data: [u8; Id::SIZE]) -> Self {
		Self(data)
	}

	pub fn test() -> Self {
		Self([0; Id::SIZE])
	}

	pub fn as_bytes(&self) -> &[u8; Id::SIZE] {
		&self.0
	}

	/// Creates a commitment by making a cryptographic digest of the state proof.
	pub fn digest_state_proof(state_proof: &StateProof) -> Self {
		let mut hasher = blake3::Hasher::new();
		bcs::serialize_into(&mut hasher, &state_proof).expect("unexpected serialization error");
		Self(hasher.finalize().into())
	}
}

impl fmt::Display for Commitment {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for byte in &self.0 {
			write!(f, "{:02x}", byte)?;
		}
		Ok(())
	}
}

impl From<Commitment> for [u8; Id::SIZE] {
	fn from(commitment: Commitment) -> [u8; Id::SIZE] {
		commitment.0
	}
}

impl From<Commitment> for Vec<u8> {
	fn from(commitment: Commitment) -> Vec<u8> {
		commitment.0.into()
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockCommitment {
	height: u64,
	block_id: Id,
	commitment: Commitment,
}

impl BlockCommitment {
	pub fn new(height: u64, block_id: Id, commitment: Commitment) -> Self {
		Self { height, block_id, commitment }
	}

	pub fn height(&self) -> u64 {
		self.height
	}

	pub fn block_id(&self) -> &Id {
		&self.block_id
	}

	pub fn commitment(&self) -> Commitment {
		self.commitment
	}

	pub fn test() -> Self {
		Self::new(0, Id::test(), Commitment::test())
	}
}

impl fmt::Display for BlockCommitment {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"BlockCommitment {{ height: {}, block_id: {}, commitment: {} }}",
			self.height, self.block_id, self.commitment
		)
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockCommitmentRejectionReason {
	InvalidBlockId,
	InvalidCommitment,
	InvalidHeight,
	ContractError,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockCommitmentEvent {
	Accepted(BlockCommitment),
	Rejected { height: u64, reason: BlockCommitmentRejectionReason },
}

pub mod test {

	#[test]
	fn test_collapse_blocks() {
		// check transactions are deduplicated
		let block1 = super::Block::test();
		let block2 = super::Block::test();

		let collapsed_block = super::Block::collapse(vec![block1.clone(), block2]);

		// assert transactions equals test transactions
		let test = super::Block::test();
		assert_eq!(collapsed_block.transactions().count(), 1);
		assert_eq!(collapsed_block.transactions().next(), test.transactions().next());

		// check transactions are preserved
		// construct a different block
		let mut diff_block1 = super::Block::test();
		let new_transaction = super::Transaction::new(vec![4, 5, 6], 0, 0);
		diff_block1.add_transaction(new_transaction.clone()).unwrap();
		let collapsed = super::Block::collapse(vec![block1, diff_block1]);
		assert_eq!(collapsed.transactions().count(), 2);
		assert_eq!(collapsed.transactions().next(), Some(&new_transaction));
		assert_eq!(collapsed.transactions().nth(1), test.transactions().next());
	}
}
