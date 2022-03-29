use super::*;

use frame_support::{assert_noop, assert_ok};
use frame_system as system;
use mock::{
	last_event, CurrenciesRegistry, Event, ExtBuilder, Origin, Runtime, System, ALICE, BOB,
};
use sp_core::H256;
use sp_runtime::traits::Hash;

fn expected() -> (<Runtime as system::Config>::Hash, CurrencyMetadata<Runtime>) {
	let metadata = CurrencyMetadata::<Runtime> {
		name: "Polkadot".into(),
		symbol: "dot".into(),
		decimals: 12_u8,
		issuer: ALICE,
	};

	let currency_id = <Runtime as system::Config>::Hashing::hash_of(&metadata);

	(currency_id, metadata)
}

#[test]
fn create_currency_works() {
	ExtBuilder.build().execute_with(|| {
		System::set_block_number(1);

		let (currency_id, metadata) = expected();

		assert_ok!(CurrenciesRegistry::create_currency(
			Origin::signed(ALICE),
			"Polkadot".into(),
			"dot".into(),
			12,
		));
		assert_eq!(
			last_event(),
			Event::CurrenciesRegistry(crate::Event::CurrencyCreated {
				currency_id,
				created_by: ALICE,
			}),
		);
		assert_eq!(CurrenciesRegistry::currencies(currency_id).unwrap(), metadata);

		assert_noop!(
			CurrenciesRegistry::create_currency(
				Origin::signed(ALICE),
				"Polkadot".into(),
				"dot".into(),
				12,
			),
			Error::<Runtime>::CurrencyExisted
		);
	});
}

#[test]
fn remove_currency_works() {
	ExtBuilder.build().execute_with(|| {
		System::set_block_number(1);

		let (currency_id, metadata) = expected();

		assert_ok!(CurrenciesRegistry::create_currency(
			Origin::signed(ALICE),
			"Polkadot".into(),
			"dot".into(),
			12,
		));

		assert_noop!(
			CurrenciesRegistry::remove_currency(Origin::signed(ALICE), H256::zero()),
			Error::<Runtime>::CurrencyNotFound
		);
		assert_noop!(
			CurrenciesRegistry::remove_currency(Origin::signed(BOB), currency_id,),
			Error::<Runtime>::NotCurrencyIssuer
		);

		assert_ok!(CurrenciesRegistry::remove_currency(Origin::signed(ALICE), currency_id));
		assert_eq!(
			last_event(),
			Event::CurrenciesRegistry(crate::Event::CurrencyRemoved {
				currency_id,
				name: metadata.name,
				symbol: metadata.symbol,
				decimals: 12,
				removed_by: ALICE,
			}),
		);

		assert_noop!(
			CurrenciesRegistry::remove_currency(Origin::signed(ALICE), currency_id),
			Error::<Runtime>::CurrencyNotFound
		);
	});
}

#[test]
fn accept_currency_works() {
	ExtBuilder.build().execute_with(|| {
		System::set_block_number(1);
		let (currency_id) = expected();

		assert_ok!(CurrenciesRegistry::create_currency(
			Origin::signed(ALICE),
			"Polkadot".into(),
			"dot".into(),
			12,
		));
		assert_noop!(
			CurrenciesRegistry::accept_currency(Origin::signed(ALICE), H256::zero()),
			Error::<Runtime>::CurrencyNotFound
		);
		assert_ok!(CurrenciesRegistry::accept_currency(Origin::signed(BOB), currency_id,));
		assert_eq!(
			last_event(),
			Event::CurrenciesRegistry(crate::Event::CurrencyAccepted {
				currency_id,
				accepted_by: BOB,
			}),
		);
		assert_eq!(CurrenciesRegistry::accepted_currencies(BOB).len(), 1);
		assert_eq!(CurrenciesRegistry::accepted_currencies(BOB)[0], currency_id);
	});
}
