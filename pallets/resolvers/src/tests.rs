#![cfg(test)]

use super::*;
use crate::pallet::ResolversNetwork as ResolversNetworkT;
use frame_support::{assert_noop, assert_ok, traits::OffchainWorker};
use mock::{
	last_event, Currencies, CurrencyId, Event, ExtBuilder, Identities, Origin, ResolversNetwork,
	Runtime, System, Timestamp, ALICE, BOB, CHARLIE, INITIAL_CREDIBILITY, UNDELEGATE_TIME,
};
use orml_traits::MultiReservableCurrency;
use pallet_identities::{IdentitiesManager, IdentityType};

pub const INIT_TIMESTAMP: u64 = 1_000;
pub const BLOCK_TIME: u64 = 6_000;

fn run_to_block_number(block_number: u64) {
	while System::block_number() < block_number {
		System::set_block_number(System::block_number() + 1);
		Timestamp::set_timestamp((System::block_number() as u64 * BLOCK_TIME) + INIT_TIMESTAMP);
		ResolversNetwork::offchain_worker(System::block_number());
	}
}

#[test]
fn join_resolvers_networks_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 300),
			Error::<Runtime>::IdentityRequired,
		);

		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));

		assert_noop!(
			ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 99),
			Error::<Runtime>::NotMeetMinimumSelfStake
		);

		assert_noop!(
			ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 10001),
			Error::<Runtime>::InsufficientBalance
		);

		assert_ok!(ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 300));

		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Candidacy);
		assert_eq!(resolver.self_stake, 300);
		assert_eq!(resolver.total_stake, 300);
		assert_eq!(resolver.delegations, [].to_vec());
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 300);

		let resolver_credibility = Identities::get_credibility(&ALICE).unwrap();
		assert_eq!(resolver_credibility, INITIAL_CREDIBILITY);

		assert_eq!(
			last_event(),
			Event::ResolversNetwork(crate::Event::ResolverCreated { account: ALICE })
		);
	});
}

#[test]
fn delegate_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 900));

		// Test delegate amount tokens that exeed the balance.
		assert_noop!(
			ResolversNetwork::delegate(Origin::signed(ALICE), CHARLIE, 1001),
			Error::<Runtime>::InsufficientBalance,
		);

		// Test delegate tokens to account not a resolver.
		assert_noop!(
			ResolversNetwork::delegate(Origin::signed(BOB), CHARLIE, 200),
			Error::<Runtime>::ResolverNotFound,
		);

		// Test an account delegate tokens to resolvers.
		assert_ok!(ResolversNetwork::delegate(Origin::signed(BOB), ALICE, 200));
		assert_eq!(
			last_event(),
			Event::ResolversNetwork(crate::Event::ResolverActivated { account: ALICE })
		);
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_eq!(resolver.self_stake, 900);
		assert_eq!(resolver.total_stake, 1100);
		assert_eq!(resolver.delegations.len(), 1);
		assert_eq!(resolver.delegations[0].delegator, BOB);
		assert_eq!(resolver.delegations[0].amount, 200);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &BOB), 200);

		// Test an account delegate more tokens to the resolver.
		assert_ok!(ResolversNetwork::delegate(Origin::signed(BOB), ALICE, 300));
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.delegations.len(), 1);
		assert_eq!(resolver.delegations[0].delegator, BOB);
		assert_eq!(resolver.delegations[0].amount, 500);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &BOB), 500);

		// Test another account delegate tokens to the resolver.
		assert_ok!(ResolversNetwork::delegate(Origin::signed(CHARLIE), ALICE, 100));
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_eq!(resolver.total_stake, 1500);
		assert_eq!(resolver.delegations.len(), 2);
		assert_eq!(resolver.delegations[0].delegator, BOB);
		assert_eq!(resolver.delegations[0].amount, 500);
		assert_eq!(resolver.delegations[1].delegator, CHARLIE);
		assert_eq!(resolver.delegations[1].amount, 100);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &CHARLIE), 100);
	});
}

#[test]
fn undelegate_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test undelegate from account that is not an resolver.
		assert_noop!(
			ResolversNetwork::undelegate(Origin::signed(BOB), CHARLIE, 200),
			Error::<Runtime>::ResolverNotFound,
		);

		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 900));

		// Test undelegate without any delegations.
		assert_noop!(
			ResolversNetwork::undelegate(Origin::signed(BOB), ALICE, 200),
			Error::<Runtime>::DelegationNotFound,
		);

		// Test undelage tokens from a resolver.
		assert_ok!(ResolversNetwork::delegate(Origin::signed(BOB), ALICE, 200));
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_ok!(ResolversNetwork::undelegate(Origin::signed(BOB), ALICE, 200));
		assert_eq!(
			last_event(),
			Event::ResolversNetwork(crate::Event::ResolverInactivated { account: ALICE })
		);
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Candidacy);
		assert_eq!(resolver.delegations.len(), 0);
		let pending_funds = ResolversNetwork::pending_funds();
		assert_eq!(pending_funds.len(), 1);
		assert_eq!(pending_funds[0].owner, BOB);
		assert_eq!(pending_funds[0].amount, 200);

		// Test undelage a small amount tokens from a resolver.
		assert_ok!(ResolversNetwork::delegate(Origin::signed(BOB), ALICE, 200));
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_eq!(resolver.delegations.len(), 1);
		assert_eq!(resolver.delegations[0].delegator, BOB);
		assert_eq!(resolver.delegations[0].amount, 200);

		assert_ok!(ResolversNetwork::undelegate(Origin::signed(BOB), ALICE, 50));
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_eq!(resolver.delegations.len(), 1);
		assert_eq!(resolver.delegations[0].delegator, BOB);
		assert_eq!(resolver.delegations[0].amount, 150);

		let pending_funds = ResolversNetwork::pending_funds();
		assert_eq!(pending_funds.len(), 2);
		assert_eq!(pending_funds[1].owner, BOB);
		assert_eq!(pending_funds[1].amount, 50);
	});
}

#[test]
fn release_funds_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 500));
		// Bob and Charlie delegate tokens.
		assert_ok!(ResolversNetwork::delegate(Origin::signed(BOB), ALICE, 200));
		assert_ok!(ResolversNetwork::delegate(Origin::signed(CHARLIE), ALICE, 200));
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &BOB), 200);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &CHARLIE), 200);
		// Bob and Charlie undelegate tokens.
		assert_ok!(ResolversNetwork::undelegate(Origin::signed(BOB), ALICE, 200));
		assert_ok!(ResolversNetwork::undelegate(Origin::signed(CHARLIE), ALICE, 200));
		// The undelegated funds are still locked.
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &BOB), 200);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &CHARLIE), 200);
		// There are 2 pending funds.
		let pending_funds = ResolversNetwork::pending_funds();
		assert_eq!(pending_funds.len(), 2);
		assert_eq!(pending_funds[0].owner, BOB);
		assert_eq!(pending_funds[0].amount, 200);
		assert_eq!(pending_funds[1].owner, CHARLIE);
		assert_eq!(pending_funds[1].amount, 200);

		// Wait until the undelegate time.
		run_to_block_number((UNDELEGATE_TIME / BLOCK_TIME).into());
		// The undelegated funds are released.
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &BOB), 0);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &CHARLIE), 0);
		// Pending funds is removed from the queue.
		let pending_funds = ResolversNetwork::pending_funds();
		assert_eq!(pending_funds.len(), 0);
	});
}

#[test]
fn resign_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test an account that is not resolver resign.
		assert_noop!(
			ResolversNetwork::resign(Origin::signed(ALICE)),
			Error::<Runtime>::NotAResolver,
		);

		// Test a resolver resign.
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(Origin::signed(ALICE), "".into(), 500));
		assert_ok!(ResolversNetwork::delegate(Origin::signed(BOB), ALICE, 200));
		assert_ok!(ResolversNetwork::delegate(Origin::signed(CHARLIE), ALICE, 200));

		assert_ok!(ResolversNetwork::resign(Origin::signed(ALICE)));
		assert_eq!(
			last_event(),
			Event::ResolversNetwork(crate::Event::ResolverTerminated { account: ALICE })
		);
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Terminated);
		assert_eq!(resolver.delegations.len(), 0);
		assert_eq!(resolver.self_stake, 0);
		assert_eq!(resolver.total_stake, 0);

		let pending_funds = ResolversNetwork::pending_funds();
		assert_eq!(pending_funds.len(), 3);
		assert_eq!(pending_funds[0].owner, ALICE);
		assert_eq!(pending_funds[0].amount, 500);
		assert_eq!(pending_funds[1].owner, BOB);
		assert_eq!(pending_funds[1].amount, 200);
		assert_eq!(pending_funds[2].owner, CHARLIE);
		assert_eq!(pending_funds[2].amount, 200);

		run_to_block_number((UNDELEGATE_TIME / BLOCK_TIME).into());

		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 0);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &BOB), 0);
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &CHARLIE), 0);
	});
}

#[test]
fn test_increase_resolver_credibility_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		// Create a active resolver.
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(ALICE),
			"".into(),
			1000
		));

		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_eq!(Identities::get_credibility(&ALICE).unwrap(), INITIAL_CREDIBILITY);

		// Test reduce a resolver credibility.
		assert_ok!(ResolversNetwork::increase_credibility(&ALICE, 10));
		assert_eq!(Identities::get_credibility(&ALICE).unwrap(), INITIAL_CREDIBILITY + 10);
	});
}

#[test]
fn test_reduce_resolver_credibility_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test a resolver resign.
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));
		assert_ok!(ResolversNetwork::join_resolvers_network(
			Origin::signed(ALICE),
			"".into(),
			1000
		));

		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(resolver.status, crate::ResolverStatus::Active);
		assert_eq!(Identities::get_credibility(&ALICE).unwrap(), INITIAL_CREDIBILITY);
		assert_ok!(ResolversNetwork::decrease_credibility(ALICE, 10));

		// Test reduce a resolver credibility.
		assert_eq!(Identities::get_credibility(&ALICE).unwrap(), INITIAL_CREDIBILITY - 10);

		// Test a resolver will be terminated if credibility under MinimumCredibility
		assert_ok!(ResolversNetwork::decrease_credibility(ALICE, 30));
		let resolver = ResolversNetwork::resolvers(ALICE).unwrap();
		assert_eq!(Identities::get_credibility(&ALICE).unwrap(), 20);
		assert_eq!(resolver.status, crate::ResolverStatus::Terminated)
	});
}
