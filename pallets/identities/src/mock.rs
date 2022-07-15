#![cfg(test)]

use crate as identities;

use frame_support::{
	construct_runtime, parameter_types,
	traits::{GenesisBuild, Nothing},
};
use frame_system as system;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
pub use pallet_balances::Call as BalancesCall;
pub use primitives::{Credibility, CurrencyId, Hash};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type BlockNumber = u64;
pub type AccountId = u128;
pub type Amount = i128;
pub type Balance = u128;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;

pub const EVALUATOR_BONDING: Balance = 100;
pub const INITIAL_CREDIBILITY: Credibility = 60;
pub const MAX_CREDIBILITY: Credibility = 100;

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
	pub const ExistentialDeposit: u128 = 1;
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
	pub const EvaluatorBonding: Balance = EVALUATOR_BONDING;
	pub const InitialCredibility: Credibility = INITIAL_CREDIBILITY;
	pub const MaxCredibility: Credibility = MAX_CREDIBILITY;
}

impl identities::Config for Runtime {
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
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Identities: identities::{Pallet, Call, Storage, Event<T>},
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
			balances: vec![(ALICE, 1_000), (BOB, 1_000), (CHARLIE, 1_000)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> { balances: self.balances }
			.assimilate_storage(&mut t)
			.unwrap();

		t.into()
	}
}

pub fn last_event() -> Event {
	system::Pallet::<Runtime>::events().pop().expect("Event expected").event
}
