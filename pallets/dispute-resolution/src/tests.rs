#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, traits::OffchainWorker};
use mock::{
	Currencies, CurrencyId, DisputeResolution, ExtBuilder, Origin,
	ResolversNetwork, Identities, Runtime, System, Timestamp, ALICE, BOB, DISPUTE_FINALIZING_TIME, LRP, RESOLVER_1, RESOLVER_2,
	RESOLVER_3,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};

pub const INIT_TIMESTAMP: u64 = 1_000;
pub const BLOCK_TIME: u64 = 6_000;

fn run_to_block_number(block_number: u64) {
	while System::block_number() < block_number {
		System::set_block_number(System::block_number() + 1);
		Timestamp::set_timestamp((System::block_number() as u64 * BLOCK_TIME) + INIT_TIMESTAMP);
		LRP::offchain_worker(System::block_number());
		ResolversNetwork::offchain_worker(System::block_number());
		DisputeResolution::offchain_worker(System::block_number());
	}
}

#[test]
fn create_dispute_works() {
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
			DisputeResolution::create_dispute(Origin::signed(ALICE), payment_hash, "".into(),),
			Error::<Runtime>::DisputeNotAccepted,
		);

		assert_ok!(LRP::accept_payment(Origin::signed(BOB), payment_hash));

		assert_ok!(DisputeResolution::create_dispute(
			Origin::signed(ALICE),
			payment_hash,
			"".into(),
		));

		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 200);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 800);

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();

		assert_eq!(dispute.payment_hash, payment_hash);
		assert_eq!(dispute.status, DisputeStatus::Finalizing);
		assert_eq!(dispute.resolvers, [].to_vec());
		assert_eq!(dispute.judgments, [].to_vec());
		assert_eq!(dispute.outcome, Judgment::ReleaseFundToPayer);

		let finalizing_disputes = DisputeResolution::finalizing_disputes();
		assert_eq!(finalizing_disputes[0], payment_hash);
	});
}

#[test]
fn fight_dispute_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Initial resolvers network.
		assert_ok!(Identities::create_identity(
			Origin::signed(RESOLVER_1),
			"Resolver 1".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(RESOLVER_1),
			"".into(),
			1100,
		));

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
		assert_ok!(DisputeResolution::create_dispute(
			Origin::signed(ALICE),
			payment_hash,
			"".into(),
		));
		assert_ok!(DisputeResolution::fight_dispute(Origin::signed(BOB), payment_hash, "".into(),));

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();

		assert_eq!(dispute.payment_hash, payment_hash);
		assert_eq!(dispute.status, DisputeStatus::Evaluating);
		assert_eq!(dispute.resolvers, [RESOLVER_1].to_vec());
		assert_eq!(dispute.judgments, [].to_vec());
		assert_eq!(dispute.outcome, Judgment::ReleaseFundToPayer);

		let finalizing_disputes = DisputeResolution::finalizing_disputes();
		assert_eq!(finalizing_disputes.len(), 0);
	});
}

#[test]
fn propose_jugdment_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Initial resolvers network.
		assert_ok!(Identities::create_identity(
			Origin::signed(RESOLVER_1),
			"Resolver 1".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(RESOLVER_1),
			"".into(),
			1100,
		));

		// Create a dispute
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
		assert_ok!(DisputeResolution::create_dispute(
			Origin::signed(ALICE),
			payment_hash,
			"".into(),
		));
		assert_ok!(DisputeResolution::fight_dispute(Origin::signed(BOB), payment_hash, "".into(),));
		assert_ok!(DisputeResolution::propose_outcome(
			Origin::signed(RESOLVER_1),
			payment_hash,
			Judgment::ReleaseFundToPayee
		));

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();

		assert_eq!(dispute.status, DisputeStatus::Finalizing);
		assert_eq!(dispute.resolvers, [RESOLVER_1].to_vec());
		assert_eq!(dispute.judgments, [(RESOLVER_1, Judgment::ReleaseFundToPayee)].to_vec());
		assert_eq!(dispute.outcome, Judgment::ReleaseFundToPayee);
	});
}

#[test]
fn escalate_dispute_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Initial resolvers network.
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(RESOLVER_1),
			"".into(),
			1100,
		));

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
		assert_ok!(DisputeResolution::create_dispute(
			Origin::signed(ALICE),
			payment_hash,
			"".into(),
		));
		assert_ok!(DisputeResolution::fight_dispute(Origin::signed(BOB), payment_hash, "".into(),));
		assert_ok!(DisputeResolution::propose_outcome(
			Origin::signed(RESOLVER_1),
			payment_hash,
			Judgment::ReleaseFundToPayee
		));
		assert_ok!(DisputeResolution::escalate_dispute(Origin::signed(ALICE), payment_hash));

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();

		assert_eq!(dispute.status, DisputeStatus::Finalizing);
		assert_eq!(dispute.outcome, Judgment::ReleaseFundToPayer);
	});
}

#[test]
fn fight_an_escalated_dispute_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Initial resolvers network.
		assert_ok!(Identities::create_identity(
			Origin::signed(RESOLVER_1),
			"Resolver 1".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(RESOLVER_1),
			"".into(),
			1100,
		));
		assert_ok!(Identities::create_identity(
			Origin::signed(RESOLVER_2),
			"Resolver 2".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(RESOLVER_2),
			"".into(),
			1100,
		));
		assert_ok!(Identities::create_identity(
			Origin::signed(RESOLVER_3),
			"Resolver 3".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(RESOLVER_3),
			"".into(),
			1100,
		));

		// Simulate escalate dispute
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
		assert_ok!(DisputeResolution::create_dispute(
			Origin::signed(ALICE),
			payment_hash,
			"".into(),
		));
		assert_ok!(DisputeResolution::fight_dispute(Origin::signed(BOB), payment_hash, "".into(),));
		assert_ok!(DisputeResolution::propose_outcome(
			Origin::signed(RESOLVER_1),
			payment_hash,
			Judgment::ReleaseFundToPayee
		));
		assert_ok!(DisputeResolution::escalate_dispute(Origin::signed(ALICE), payment_hash));
		assert_ok!(DisputeResolution::fight_dispute(Origin::signed(BOB), payment_hash, "".into()));

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();

		assert_eq!(dispute.status, DisputeStatus::Evaluating);
		assert_eq!(dispute.resolvers.len(), 3);
		assert_eq!(dispute.judgments, [(RESOLVER_1, Judgment::ReleaseFundToPayee)].to_vec());
		assert_eq!(dispute.outcome, Judgment::ReleaseFundToPayer);

		assert_ok!(DisputeResolution::propose_outcome(
			Origin::signed(RESOLVER_2),
			payment_hash,
			Judgment::ReleaseFundToPayee
		));

		assert_ok!(DisputeResolution::propose_outcome(
			Origin::signed(RESOLVER_3),
			payment_hash,
			Judgment::ReleaseFundToPayee
		));

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();

		assert_eq!(dispute.status, DisputeStatus::Finalizing);
		assert_eq!(dispute.resolvers.len(), 3);
		assert_eq!(dispute.judgments.len(), 3);
		assert_eq!(dispute.outcome, Judgment::ReleaseFundToPayee);
	});
}

#[test]
fn finalize_judgment_works() {
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
		assert_ok!(DisputeResolution::create_dispute(
			Origin::signed(ALICE),
			payment_hash,
			"".into(),
		));

		run_to_block_number((DISPUTE_FINALIZING_TIME / BLOCK_TIME + 1).into());

		let finalizing_disputes = DisputeResolution::finalizing_disputes();
		assert_eq!(finalizing_disputes.len(), 0);

		let dispute = DisputeResolution::disputes(&payment_hash).unwrap();
		assert_eq!(dispute.status, DisputeStatus::Resolved);

		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 1000);
	});
}
