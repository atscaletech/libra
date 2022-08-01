#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, traits::{ OffchainWorker }};
use frame_system as system;
use mock::{
	last_event, Currencies, CurrencyId, Event, ExtBuilder, Origin, Runtime, System, ALICE, BOB,
	CHARLIE, LRP, Timestamp, PENDING_PAYMENT_WAITING_TIME, FULFILLED_WAITING_TIME,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
use sp_runtime::traits::Hash;

pub const INIT_TIMESTAMP: u64 = 1_000;
pub const BLOCK_TIME: u64 = 6_000;

fn run_to_block_number(block_number: u64) {
	while System::block_number() < block_number {
		System::set_block_number(System::block_number() + 1);
		Timestamp::set_timestamp((System::block_number() as u64 * BLOCK_TIME) + INIT_TIMESTAMP);
		LRP::offchain_worker(System::block_number());
	}
}

#[test]
fn create_payment_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			LRP::create_payment(
				Origin::signed(ALICE),
				BOB,
				1001,
				CurrencyId::Native,
				"".into(),
				"".into(),
			),
			Error::<Runtime>::InsufficientBalance
		);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));

		let payment_hashes = LRP::payments_owned(&ALICE);
		assert_eq!(payment_hashes.len(), 1);
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentCreated {
				payment_hash: payment_hashes[0],
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 100);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 900);

		let payment = LRP::payments(payment_hashes[0]).unwrap();
		assert_eq!(payment.payer, ALICE);
		assert_eq!(payment.payee, BOB);
		assert_eq!(payment.amount, 100);
		assert_eq!(payment.currency_id, CurrencyId::Native);
		assert_eq!(payment.description, "".as_bytes());
		assert_eq!(payment.receipt_hash,  <Runtime as system::Config>::Hashing::hash_of(&"".as_bytes()));
		assert_eq!(payment.status, PaymentStatus::Pending);
	});
}

#[test]
fn accept_payment_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hashes = LRP::payments_owned(&ALICE);

		assert_eq!(payment_hashes.len(), 1);
		assert_noop!(
			LRP::accept_payment(Origin::signed(CHARLIE), payment_hashes[0]),
			Error::<Runtime>::AccessDenied
		);

		assert_ok!(LRP::accept_payment(Origin::signed(BOB), payment_hashes[0]));
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentAccepted {
				payment_hash: payment_hashes[0],
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);

		let payment = LRP::payments(payment_hashes[0]).unwrap();
		assert_eq!(payment.status, crate::PaymentStatus::Accepted);
	});
}

#[test]
fn reject_payment_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hashes = LRP::payments_owned(&ALICE);

		assert_eq!(payment_hashes.len(), 1);
		assert_noop!(
			LRP::reject_payment(Origin::signed(CHARLIE), payment_hashes[0]),
			Error::<Runtime>::AccessDenied
		);
		assert_noop!(
			LRP::reject_payment(Origin::signed(ALICE), payment_hashes[0]),
			Error::<Runtime>::AccessDenied
		);

		assert_ok!(LRP::reject_payment(Origin::signed(BOB), payment_hashes[0]));
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentRejected {
				payment_hash: payment_hashes[0],
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);

		let payment = LRP::payments(payment_hashes[0]).unwrap();
		assert_eq!(payment.status, crate::PaymentStatus::Rejected);

		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 1000);
	});
}

#[test]
fn auto_expire_payment_by_offchain_worker_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));

		run_to_block_number((PENDING_PAYMENT_WAITING_TIME / BLOCK_TIME).into());

		let payment_hash = LRP::payments_owned(&ALICE)[0];
		let payment = LRP::payments(payment_hash).unwrap();

		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentExpired {
				payment_hash,
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);
		assert_eq!(payment.status, PaymentStatus::Expired);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 1_000);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &BOB), 1_000);
	});
}

#[test]
fn cancel_payment_works_with_payer() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hash = LRP::payments_owned(&ALICE)[0];

		assert_noop!(
			LRP::cancel_payment(Origin::signed(CHARLIE), payment_hash),
			Error::<Runtime>::AccessDenied
		);

		assert_ok!(LRP::cancel_payment(Origin::signed(ALICE), payment_hash));
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentCancelled {
				payment_hash,
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 1000);
	});
}

#[test]
fn cancel_payment_only_works_with_payee_if_payment_accepted() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hash = LRP::payments_owned(&ALICE)[0];
		assert_ok!(LRP::accept_payment(Origin::signed(BOB), payment_hash));
		assert_noop!(
			LRP::cancel_payment(Origin::signed(ALICE), payment_hash),
			Error::<Runtime>::AccessDenied
		);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 100);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 900);

		assert_ok!(LRP::cancel_payment(Origin::signed(BOB), payment_hash));
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentCancelled {
				payment_hash,
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 1000);
	});
}

#[test]
fn full_fill_payment_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hash = LRP::payments_owned(&ALICE)[0];
		assert_ok!(LRP::accept_payment(Origin::signed(BOB), payment_hash));

		assert_noop!(
			LRP::fulfill_payment(Origin::signed(CHARLIE), payment_hash),
			Error::<Runtime>::AccessDenied
		);

		assert_ok!(LRP::fulfill_payment(Origin::signed(BOB), payment_hash));
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentFulfilled {
				payment_hash,
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);

		let payment = LRP::payments(payment_hash).unwrap();
		assert_eq!(payment.status, crate::PaymentStatus::Fulfilled);
	});
}

#[test]
fn complete_payment_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hash = LRP::payments_owned(&ALICE)[0];

		assert_ok!(LRP::accept_payment(Origin::signed(BOB), payment_hash));
		assert_ok!(LRP::fulfill_payment(Origin::signed(BOB), payment_hash));

		assert_noop!(
			LRP::complete_payment(Origin::signed(CHARLIE), payment_hash),
			Error::<Runtime>::AccessDenied
		);

		assert_noop!(
			LRP::complete_payment(Origin::signed(BOB), payment_hash),
			Error::<Runtime>::AccessDenied
		);

		assert_ok!(LRP::complete_payment(Origin::signed(ALICE), payment_hash));
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentCompleted {
				payment_hash,
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 900);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &BOB), 1100);
	});
}

#[test]
fn auto_complete_payment_by_offchain_worker_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(LRP::create_payment(
			Origin::signed(ALICE),
			BOB,
			100,
			CurrencyId::Native,
			"".into(),
			"".into(),
		));
		let payment_hash = LRP::payments_owned(&ALICE)[0];
	
		assert_ok!(LRP::accept_payment(Origin::signed(BOB), payment_hash));
		assert_ok!(LRP::fulfill_payment(Origin::signed(BOB), payment_hash));

		run_to_block_number((FULFILLED_WAITING_TIME / BLOCK_TIME).into());

		let payment_hash = LRP::payments_owned(&ALICE)[0];
		let payment = LRP::payments(payment_hash).unwrap();

		assert_eq!(payment.status, PaymentStatus::Completed);
		assert_eq!(
			last_event(),
			Event::LRP(crate::Event::PaymentCompleted {
				payment_hash,
				payer: ALICE,
				payee: BOB,
				currency_id: CurrencyId::Native,
				amount: 100,
			}),
		);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 900);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &BOB), 1100);
	});
}
