#![cfg(test)]

use crate as dispute_resolution;

use frame_support::{
	construct_runtime, parameter_types,
	traits::{GenesisBuild, Nothing},
};
use frame_system as system;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
pub use pallet_balances::Call as BalancesCall;
use pallet_identities;
use pallet_lrp;
use pallet_resolvers;
use pallet_timestamp::{self as timestamp};
pub use primitives::{Credibility, CurrencyId, Hash};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type BlockNumber = u64;
pub type AccountId = u128;
pub type Amount = i128;
pub type Balance = u128;
pub type Moment = u64;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const RESOLVER_1: AccountId = 4;
pub const RESOLVER_2: AccountId = 5;
pub const RESOLVER_3: AccountId = 6;

pub const PENDING_PAYMENT_WAITING_TIME: Moment = 172800000;
pub const FULFILLED_WAITING_TIME: Moment = 2592000000;

// Identities pallet config
pub const EVALUATOR_BONDING: Balance = 1000;
pub const INITIAL_CREDIBILITY: Credibility = 60;
pub const MAX_CREDIBILITY: Credibility = 100;

// Resolvers network config
pub const PENALTY_TOKEN_LOCK_TIME: Moment = 172800000;
pub const UNDELEGATE_TIME: Moment = 172800000;
pub const MINIMUM_SELF_STAKE: Balance = 100;
pub const ACTIVATION_STAKE_AMOUNT: Balance = 1000;
pub const REQUIRED_CREDIBILITY: Credibility = 30;

// pub const DISPUTE_FINALIZING_TIME: Moment = 2592000000;
pub const DISPUTE_FINALIZING_TIME: Moment = 10_000;
pub const DISPUTE_FEE: Balance = 100;
pub const CREDIBILITY_GAIN: Credibility = 1;
pub const CREDIBILITY_LOSS: Credibility = 10;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const MinimumPeriod: Moment = 1000;
}

impl timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId<Hash>| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId<Hash>;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = Nothing;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId<Hash> = CurrencyId::<Hash>::Native;
}

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const BondingAmount: Balance = 100_000_000_000_000;
}

impl currencies_registry::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type BondingAmount = BondingAmount;
}

parameter_types! {
	pub const PendingPaymentWaitingTime: Moment = PENDING_PAYMENT_WAITING_TIME;
	pub const FulfilledPaymentWaitingTime: Moment = FULFILLED_WAITING_TIME;
}

impl pallet_lrp::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type CurrenciesManager = CurrenciesRegistry;
	type PendingPaymentWaitingTime = PendingPaymentWaitingTime;
	type FulfilledPaymentWaitingTime = FulfilledPaymentWaitingTime;
}

parameter_types! {
	pub const PenaltyTokenLockTime: Moment = PENALTY_TOKEN_LOCK_TIME;
	pub const UndelegateTime: Moment = UNDELEGATE_TIME;
	pub const MinimumSelfStake: Balance = MINIMUM_SELF_STAKE;
	pub const ActivationStakeAmount: Balance = ACTIVATION_STAKE_AMOUNT;
	pub const RequiredCredibility: Credibility = REQUIRED_CREDIBILITY;
}

impl pallet_resolvers::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type IdentitiesManager = Identities;
	type Randomness = RandomnessCollectiveFlip;
	type PenaltyTokenLockTime = PenaltyTokenLockTime;
	type MinimumSelfStake = MinimumSelfStake;
	type ActivationStakeAmount = ActivationStakeAmount;
	type UndelegateTime = UndelegateTime;
	type RequiredCredibility = RequiredCredibility;
}

parameter_types! {
	pub const DisputeFinalizingTime: Moment = DISPUTE_FINALIZING_TIME;
	pub const DisputeFee: Balance = DISPUTE_FEE;
	pub const CredibilityGain: Credibility = CREDIBILITY_GAIN;
	pub const CredibilityLoss: Credibility = CREDIBILITY_LOSS;
}

impl dispute_resolution::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type PaymentProtocol = LRP;
	type ResolversNetwork = ResolversNetwork;
	type IdentitiesManager = Identities;
	type DisputeFinalizingTime = DisputeFinalizingTime;
	type DisputeFee = DisputeFee;
	type CredibilityGain = CredibilityGain;
	type CredibilityLoss = CredibilityLoss;
}

parameter_types! {
	pub const EvaluatorBonding: Balance = EVALUATOR_BONDING;
	pub const InitialCredibility: Credibility = INITIAL_CREDIBILITY;
	pub const MaxCredibility: Credibility = MAX_CREDIBILITY;
}

impl pallet_identities::Config for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type EvaluatorBonding = EvaluatorBonding;
	type InitialCredibility = InitialCredibility;
	type MaxCredibility = MaxCredibility;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: timestamp::{Pallet, Call, Storage, Inherent},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		CurrenciesRegistry: currencies_registry::{Pallet, Call, Storage, Event<T>},
		Identities: pallet_identities::{Pallet, Call, Storage, Event<T>},
		ResolversNetwork: pallet_resolvers::{Pallet, Call, Storage, Event<T>},
		LRP: pallet_lrp::{Pallet, Call, Storage, Event<T>},
		DisputeResolution: dispute_resolution::{Pallet, Call, Storage, Event<T>},
	}
);

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId<Hash>, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			balances: vec![(ALICE, CurrencyId::Native, 1_000), (BOB, CurrencyId::Native, 1_000)],
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![
				(ALICE, 1_000),
				(BOB, 1_000),
				(RESOLVER_1, 2_000),
				(RESOLVER_2, 2_000),
				(RESOLVER_3, 2_000),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> { balances: self.balances }
			.assimilate_storage(&mut t)
			.unwrap();

		t.into()
	}
}
