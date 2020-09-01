//! The Substrate Node Banana runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs")); //

#[cfg(feature = "std")]
/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect("Development wasm binary is not available. This means the client is \
						built with `BUILD_DUMMY_WASM_BINARY` flag and it is only usable for \
						production chains. Please rebuild with the flag disabled.")
} //

use sp_std::{prelude::*, marker::PhantomData};
use codec::{Encode, Decode}; //
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, U256, H160, H256, u32_trait::{_1, _2, _3, _4},}; //
use sp_runtime::{
	ApplyExtrinsicResult, generic, create_runtime_str, impl_opaque_keys, MultiSignature,
	transaction_validity::{TransactionValidity, TransactionSource, TransactionPriority},
}; //

use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, IdentityLookup, Verify, IdentifyAccount, NumberFor, Saturating,
	self, StaticLookup, SaturatedConversion, ConvertInto, OpaqueKeys
}; //

// pub use node_primitives::{AccountId, Signature}; //
// use node_primitives::{AccountIndex, Balance, BlockNumber, Hash, Index, Moment}; //

pub use banana_primitives::{
	AccountId, AccountIndex, Balance, BlockNumber, Hash, Index, Moment, Signature, DigestItem
};

use sp_api::impl_runtime_apis; //
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId; //
pub use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList}; //
pub use pallet_grandpa::fg_primitives; //
use sp_version::RuntimeVersion; //
#[cfg(feature = "std")]
use sp_version::NativeVersion; //
use sp_core::crypto::Public;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage; //
pub use pallet_timestamp::Call as TimestampCall;
pub use pallet_balances::Call as BalancesCall; //
use sp_runtime::{Permill, Perbill, Perquintill, Percent, ModuleId, FixedPointNumber}; //
use sp_runtime::curve::PiecewiseLinear; //
use static_assertions::const_assert; //

#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall; //
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus; //

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls; //
use impls::{CurrencyToVoteHandler, Author}; //

/// Constant values used within the runtime.
pub mod constants; //
use constants::{time::*, currency::*}; //
use sp_runtime::generic::Era; //

/// Weights for pallets used in the runtime.
// mod weights;

pub use frame_support::{
	construct_runtime, parameter_types, StorageValue, debug, RuntimeDebug,
	traits::{KeyOwnerProofSystem, Randomness, FindAuthor, Currency, Imbalance, OnUnbalanced, LockIdentifier},
	weights::{
		Weight, IdentityFee,
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
	},
	ConsensusEngineId,
}; //

use pallet_im_online::ed25519::AuthorityId as ImOnlineId; //
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId; //
pub use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo; //
pub use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment}; //

use pallet_session::{historical as pallet_session_historical}; //
use sp_inherents::{InherentData, CheckInherentsResult}; //

use frame_ethereum::{Block as EthereumBlock, Transaction as EthereumTransaction, Receipt as EthereumReceipt};
use frame_evm::{Account as EVMAccount, FeeCalculator, HashedAddressMapping, EnsureAddressTruncated};
use frontier_rpc_primitives::{TransactionStatus};

use frame_system::{EnsureRoot, EnsureOneOf}; //
use frame_support::traits::InstanceFilter; //

// pub use pallet_contracts_rpc_runtime_api::ContractExecResult; //

// Declare under banana_primitive
// pub type BlockNumber = constants::time::BlockNumber;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
// pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
// pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
// pub type AccountIndex = u32;

/// Balance of an account.
//pub type Balance = constants::currency::Balance;

/// Index of a transaction in the chain.
// pub type Index = u32;

/// A hash of some data used by the chain.
// pub type Hash = sp_core::H256;

/// Digest item type.
// pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
	
		}
	}
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-banana"),
	impl_name: create_runtime_str!("node-banana"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
}; //

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
} //

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance; //

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item=NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 80% to treasury, 20% to author
			let mut split = fees.ration(80, 20);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 80% to treasury, 20% to author (though this can be anything)
				tips.ration_merge_into(80, 20, &mut split);
			}
			Treasury::on_unbalanced(split.0);
			Author::on_unbalanced(split.1);
		}
	}
} //

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
		.saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MaximumBlockWeight::get();
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Trait for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = (); //
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId; //
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call; //
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	// type Lookup = IdentityLookup<AccountId>;
	type Lookup = Indices;
	// type Lookup = Indices;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index; //
	/// The index type for blocks.
	type BlockNumber = BlockNumber; //
	/// The type for hashing blocks and tries.
	type Hash = Hash; //
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256; //
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>; //
	/// The ubiquitous event type.
	type Event = Event; //
	/// The ubiquitous origin type.
	type Origin = Origin; //
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount; //
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight; //
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight; //
	/// The weight of the overhead invoked on the block import process, independent of the
	/// extrinsics included in that block.
	type BlockExecutionWeight = BlockExecutionWeight; //
	/// The base weight of any extrinsic processed by the runtime, independent of the
	/// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight; //
	/// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
	/// idependent of the logic of that extrinsics. (Roughly max block weight - average on
	/// initialize cost).
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight; //
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength; //
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio; //
	/// Version of the runtime.
	type Version = Version; //
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type ModuleToIndex = ModuleToIndex; // 
	/// What to do if a new account is created.
	type OnNewAccount = (); //
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = (); //
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();

	type MigrateAccount = (Balances, Identity, Democracy, Elections, ImOnline, Recovery, Staking, Session, Vesting); //
} //

impl pallet_utility::Trait for Runtime {
	type Event = Event;
	type Call = Call;
} //

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
} //

impl pallet_multisig::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
} //

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
} //
impl Default for ProxyType { fn default() -> Self { Self::Any } } //
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(c,
				Call::Balances(..) | Call::Vesting(pallet_vesting::Call::vested_transfer(..))
					| Call::Indices(pallet_indices::Call::transfer(..))
			),
			ProxyType::Governance => matches!(c,
				Call::Democracy(..) | Call::Council(..) | Call::Elections(..) | Call::Treasury(..)
			),
			ProxyType::Staking => matches!(c, Call::Staking(..)),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
} //

impl pallet_proxy::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
} //

impl pallet_scheduler::Trait for Runtime {
	type Event = Event;
	type Origin = Origin;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
} //

impl pallet_aura::Trait for Runtime {
	type AuthorityId = AuraId;
} //

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
} //

impl pallet_indices::Trait for Runtime {
	type AccountIndex = AccountIndex;
	type Event = Event;
	type Currency = Balances;
	type Deposit = IndexDeposit;
} //

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
	type Call = Call;

	//type KeyOwnerProofSystem = ();
	type KeyOwnerProofSystem = Historical;

	// type KeyOwnerProof =
	//	<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;


	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;

	// type HandleEquivocation = ();
	type HandleEquivocation = pallet_grandpa::EquivocationHandler<
		Self::KeyOwnerIdentification,
		edgeware_primitives::report::ReporterAppCrypto,
		Runtime,
		Offences,
	>;

} //

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
} //

impl pallet_timestamp::Trait for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
} //

parameter_types! {
	pub const ExistentialDeposit: u128 = 1 * MILLICENTS;
}//

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	//type AccountStore = System;
	type AccountStore = frame_system::Module<Runtime>;
	type WeightInfo = ();
} //

//parameter_types! {
//	pub const TransactionByteFee: Balance = 1;
//}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
} //

impl pallet_transaction_payment::Trait for Runtime {
	type Currency = Balances;
	// type OnTransactionPayment = ();
	type OnTransactionPayment = DealWithFees;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	// type FeeMultiplierUpdate = ();
	type FeeMultiplierUpdate =
		TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

} //

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
} //

impl pallet_authorship::Trait for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (Staking, ImOnline);
} //

parameter_types! {
	pub const Period: BlockNumber = 1 * HOURS;
	pub const Offset: BlockNumber = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
} //

impl pallet_session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Trait>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = Staking;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
} //

impl pallet_session::historical::Trait for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
} //

pallet_staking_reward_curve::build! {
	const CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
} //

parameter_types! {
	// 1 hour session, 6 hour era
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;
	// 2 * 28 eras * 6 hours/era = 14 day bonding duration
	pub const BondingDuration: pallet_staking::EraIndex = 2 * 28;
	// 28 eras * 6 hours/era = 7 day slash duration
	pub const SlashDeferDuration: pallet_staking::EraIndex = 28;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &CURVE;
	pub const ElectionLookahead: BlockNumber = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const MaxNominatorRewardedPerValidator: u32 = 128;
	pub const MaxIterations: u32 = 5;
	// 0.05%. The higher the value, the more strict solution acceptance becomes.
	pub MinSolutionScoreBump: Perbill = Perbill::from_rational_approximation(5u32, 10_000);
} //

impl pallet_staking::Trait for Runtime {
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVoteHandler;
	type RewardRemainder = Treasury;
	type Event = Event;
	type Slash = Treasury; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = EnsureOneOf<
		AccountId,
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>
	>;
	type SessionInterface = Self;
	type RewardCurve = RewardCurve;
	type NextNewSession = Session;
	type ElectionLookahead = ElectionLookahead;
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MinSolutionScoreBump = MinSolutionScoreBump;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = StakingUnsignedPriority;
} //

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 7 * 24 * 60 * MINUTES;
	pub const VotingPeriod: BlockNumber = 7 * 24 * 60 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const InstantAllowed: bool = false;
	pub const EnactmentPeriod: BlockNumber = 8 * 24 * 60 * MINUTES;
	pub const CooloffPeriod: BlockNumber = 7 * 24 * 60 * MINUTES;
	pub const PreimageByteDeposit: Balance = 1 * CENTS;
	pub const MaxVotes: u32 = 100;
} //

impl pallet_democracy::Trait for Runtime {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>;
	/// A 60% super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = frame_system::EnsureOneOf<AccountId,
		pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
		frame_system::EnsureRoot<AccountId>,
	>;
	/// Three fourths of the committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = frame_system::EnsureOneOf<AccountId,
		pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>,
		frame_system::EnsureRoot<AccountId>,
	>;
	type InstantOrigin = frame_system::EnsureNever<AccountId>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = frame_system::EnsureOneOf<AccountId,
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>,
		frame_system::EnsureRoot<AccountId>,
	>;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = frame_system::EnsureOneOf<AccountId,
		pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>,
		frame_system::EnsureRoot<AccountId>,
	>;
	// No vetoing
	type VetoOrigin = frame_system::EnsureNever<AccountId>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type MaxVotes = MaxVotes;
} //

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 14 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
} //

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Trait<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
} //

parameter_types! {
	pub const CandidacyBond: Balance = 1_000 * DOLLARS;
	pub const VotingBond: Balance = 10 * DOLLARS;
	pub const TermDuration: BlockNumber = 28 * DAYS;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 7;
	pub const ElectionsPhragmenModuleId: LockIdentifier = *b"phrelect";
} //

// Make sure that there are no more than `MAX_MEMBERS` members elected via phragmen.
const_assert!(DesiredMembers::get() <= pallet_collective::MAX_MEMBERS); //

impl pallet_elections_phragmen::Trait for Runtime {
	type ModuleId = ElectionsPhragmenModuleId;
	type Event = Event;
	type Currency = Balances;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = CurrencyToVoteHandler;
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type LoserCandidate = ();
	type BadReport = ();
	type KickedMember = ();
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
} //

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1_000 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 14 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const TipReportDepositPerByte: Balance = 1 * CENTS;
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
} //

impl pallet_treasury::Trait for Runtime {
	type Currency = Balances;
	type ApproveOrigin = EnsureOneOf<
		AccountId,
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>
	>;
	type RejectOrigin = EnsureOneOf<
		AccountId,
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>
	>;
	type Tippers = Elections;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type TipReportDepositPerByte = TipReportDepositPerByte;
	type Event = Event;
	type ProposalRejection = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type ModuleId = TreasuryModuleId;
} //

parameter_types! {
	pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	/// We prioritize im-online heartbeats over phragmen solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
} //

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(
		Call,
		<UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload,
	)> {
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			pallet_grandpa::ValidateEquivocationReport::<Runtime>::new(),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				debug::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature.into(), extra)))
	}
} //

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
} //

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
} //

impl pallet_im_online::Trait for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type SessionDuration = SessionDuration;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
} //

parameter_types! {
	pub OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) * MaximumBlockWeight::get();
} //

impl pallet_offences::Trait for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
	type WeightSoftLimit = OffencesWeightSoftLimit;
} //

impl pallet_authority_discovery::Trait for Runtime {} //

parameter_types! {
	pub const WindowSize: BlockNumber = 101;
	pub const ReportLatency: BlockNumber = 1000;
} //

impl pallet_finality_tracker::Trait for Runtime {
	type OnFinalizationStalled = ();
	type WindowSize = WindowSize;
	type ReportLatency = ReportLatency;
} //

parameter_types! {
	pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
	pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
	pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
} //

impl pallet_identity::Trait for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type RegistrarOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
} //

parameter_types! {
	pub const ConfigDepositBase: Balance = 5 * DOLLARS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 5 * DOLLARS;
} //

impl pallet_recovery::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
} //

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * DOLLARS;
	// pub const EVMModuleId: ModuleId = ModuleId(*b"py/evmpa");
} //

impl pallet_vesting::Trait for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
} //

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
} //

impl signaling::Trait for Runtime {
	type Event = Event;
	type Currency = Balances;
} //

impl treasury_reward::Trait for Runtime {
	type Event = Event;
	type Currency = Balances;
} //

impl voting::Trait for Runtime {
	type Event = Event;
} //

/// Fixed gas price of `1`.
pub struct FixedGasPrice;

impl FeeCalculator for FixedGasPrice {
	fn min_gas_price() -> U256 {
		// Gas price is always one token per gas.
		1.into()
	}
}

parameter_types! {
	pub const ChainId: u64 = 48;
}

impl frame_evm::Trait for Runtime {
	type FeeCalculator = FixedGasPrice;
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;
	type Currency = Balances;
	type Event = Event;
	type Precompiles = ();
	type ChainId = ChainId;
}

pub struct EthereumFindAuthor<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F>
{
	fn find_author<'a, I>(digests: I) -> Option<H160> where
		I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])>
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Aura::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
		}
		None
	}
}

impl frame_ethereum::Trait for Runtime {
	type Event = Event;
	type FindAuthor = EthereumFindAuthor<Aura>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		// NodeBlock = opaque::Block,
		NodeBlock = banana_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Utility: pallet_utility::{Module, Call, Event},
		Aura: pallet_aura::{Module, Config<T>, Inherent(Timestamp)},
		
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Authorship: pallet_authorship::{Module, Call, Storage, Inherent}, 
		Indices: pallet_indices::{Module, Call, Storage, Config<T>, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},

		Staking: pallet_staking::{Module, Call, Config<T>, Storage, Event<T>, ValidateUnsigned},
		Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
		Democracy: pallet_democracy::{Module, Call, Storage, Config, Event<T>},
		Council: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		Elections: pallet_elections_phragmen::{Module, Call, Storage, Event<T>, Config<T>},

		FinalityTracker: pallet_finality_tracker::{Module, Call, Inherent},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
		Treasury: pallet_treasury::{Module, Call, Storage, Config, Event<T>},
		
		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
		ImOnline: pallet_im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
		AuthorityDiscovery: pallet_authority_discovery::{Module, Call, Config},
		Offences: pallet_offences::{Module, Call, Storage, Event},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		Identity: pallet_identity::{Module, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},
		Recovery: pallet_recovery::{Module, Call, Storage, Event<T>},
		Vesting: pallet_vesting::{Module, Call, Storage, Event<T>, Config<T>},
				
		Ethereum: frame_ethereum::{Module, Call, Storage, Event, Config, ValidateUnsigned},
		EVM: frame_evm::{Module, Config, Call, Storage, Event<T>},
		
		Historical: pallet_session_historical::{Module},
		Proxy: pallet_proxy::{Module, Call, Storage, Event<T>},
		Multisig: pallet_multisig::{Module, Call, Storage, Event<T>},

		Signaling: signaling::{Module, Call, Storage, Config<T>, Event<T>},
		Voting: voting::{Module, Call, Storage, Event<T>},
		TreasuryReward: treasury_reward::{Module, Call, Storage, Config<T>, Event<T>},

	}
);

pub struct TransactionConverter;

impl frontier_rpc_primitives::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: frame_ethereum::Transaction) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_unsigned(frame_ethereum::Call::<Runtime>::transact(transaction).into())
	}
}

impl frontier_rpc_primitives::ConvertTransaction<opaque::UncheckedExtrinsic> for TransactionConverter {
	fn convert_transaction(&self, transaction: frame_ethereum::Transaction) -> opaque::UncheckedExtrinsic {
		let extrinsic = UncheckedExtrinsic::new_unsigned(frame_ethereum::Call::<Runtime>::transact(transaction).into());
		let encoded = extrinsic.encode();
		opaque::UncheckedExtrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
	}
}

/// The address format for describing accounts.
// pub type Address = AccountId;
pub type Address = <Indices as StaticLookup>::Source; //
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	pallet_grandpa::ValidateEquivocationReport<Runtime>,
); //

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>; //
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;

/// Custom runtime upgrade to execute the balances migration before the account migration.
mod custom_migration {
	use super::*;

	use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
	use pallet_balances::migration::on_runtime_upgrade as balances_upgrade;
	use frame_system::migration::migrate_accounts as accounts_upgrade;
	use pallet_staking::migration::migrate_to_simple_payouts as staking_upgrade;

	pub struct Upgrade;
	impl OnRuntimeUpgrade for Upgrade {
		fn on_runtime_upgrade() -> Weight {
			let mut weight = 0;
			weight += balances_upgrade::<Runtime, pallet_balances::DefaultInstance>();
			weight += staking_upgrade::<Runtime>();
			weight += accounts_upgrade::<Runtime>();
			weight
		}
	}
} //

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllModules,
	custom_migration::Upgrade
>; //

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> u64 {
			Aura::slot_duration()
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl frontier_rpc_primitives::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			EVM::account_basic(&address)
		}

		fn gas_price() -> U256 {
			FixedGasPrice::min_gas_price()
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			EVM::account_codes(address)
		}

		fn author() -> H160 {
			<frame_ethereum::Module<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			EVM::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
			action: frame_ethereum::TransactionAction,
		) -> Option<(Vec<u8>, U256)> {
			match action {
				frame_ethereum::TransactionAction::Call(to) =>
					EVM::execute_call(
						from,
						to,
						data,
						value,
						gas_limit.low_u32(),
						gas_price,
						nonce,
						false,
					).ok().map(|(_, ret, gas)| (ret, gas)),
				frame_ethereum::TransactionAction::Create =>
					EVM::execute_create(
						from,
						data,
						value,
						gas_limit.low_u32(),
						gas_price,
						nonce,
						false,
					).ok().map(|(_, _, gas)| (vec![], gas)),
			}
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<frame_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<frame_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
		UncheckedExtrinsic,
	> for Runtime {
		fn query_info(
			//uxt: <Block as BlockT>::Extrinsic,
			uxt: UncheckedExtrinsic
			len: u32
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
	} //

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			// None
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_report_equivocation_extrinsic(
				equivocation_proof,
				key_owner_proof,
			)

		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			// None
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)

		}
	} //

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
 	} //

}
