#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	last_event, Currencies, CurrencyId, Event, ExtBuilder, Identities, Origin, Runtime, System,
	ALICE, BOB, CHARLIE,
};
use orml_traits::{MultiCurrency, MultiReservableCurrency};

#[test]
fn create_identity_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test create identity without any data.
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));

		assert_eq!(
			last_event(),
			Event::Identities(crate::Event::IdentityCreated { account_id: ALICE })
		);

		let identity = Identities::identities(&ALICE).unwrap();

		assert_eq!(identity.name, "Alice".as_bytes());
		assert_eq!(identity.data.len(), 0);
		assert_eq!(identity.identity_type, IdentityType::Individual);
		assert_eq!(identity.reviews.len(), 0);

		// Test identity just can create one time.
		assert_noop!(
			Identities::create_identity(
				Origin::signed(ALICE),
				"Alice".into(),
				IdentityType::Individual,
				[].into(),
			),
			Error::<Runtime>::IdentityExisted
		);

		// Test create identity with valid email and valid domain.
		assert_ok!(Identities::create_identity(
			Origin::signed(BOB),
			"Bob".into(),
			IdentityType::Individual,
			[
				IdentityFieldInput {
					name: "domain".into(),
					value: "atscale.xyz".into(),
					verify_method: VerifyMethod::Domain,
				},
				IdentityFieldInput {
					name: "email".into(),
					value: "hello@atscale.xyz".into(),
					verify_method: VerifyMethod::Email,
				}
			]
			.into(),
		));

		let identity = Identities::identities(&BOB).unwrap();

		assert_eq!(identity.name, "Bob".as_bytes());
		assert_eq!(identity.identity_type, IdentityType::Individual);
		assert_eq!(identity.reviews.len(), 0);
		assert_eq!(identity.data.len(), 2);
		assert_eq!(identity.data[0].name, "domain".as_bytes());
		assert_eq!(identity.data[0].value, "atscale.xyz".as_bytes());
		assert_eq!(identity.data[1].name, "email".as_bytes());
		assert_eq!(identity.data[1].value, "hello@atscale.xyz".as_bytes());

		// Test domain validation
		assert_noop!(
			Identities::create_identity(
				Origin::signed(CHARLIE),
				"Charlie".into(),
				IdentityType::Individual,
				[IdentityFieldInput {
					name: "domain".into(),
					value: "notadomain".into(),
					verify_method: VerifyMethod::Domain,
				},]
				.into(),
			),
			Error::<Runtime>::InvalidDomain
		);

		// Test email validation
		assert_noop!(
			Identities::create_identity(
				Origin::signed(CHARLIE),
				"Charlie".into(),
				IdentityType::Individual,
				[IdentityFieldInput {
					name: "email".into(),
					value: "notanemail".into(),
					verify_method: VerifyMethod::Email,
				},]
				.into(),
			),
			Error::<Runtime>::InvalidEmail,
		);
	});
}

#[test]
fn update_identity_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));

		assert_ok!(Identities::update_identity(
			Origin::signed(ALICE),
			Some("NewAlice".into()),
			None,
		));
		assert_eq!(
			last_event(),
			Event::Identities(crate::Event::IdentityUpdated { account_id: ALICE })
		);

		let identity = Identities::identities(&ALICE).unwrap();
		assert_eq!(identity.name, "NewAlice".as_bytes());

		// Test update identity data
		assert_ok!(Identities::update_identity(
			Origin::signed(ALICE),
			None,
			Some(
				[IdentityFieldInput {
					name: "email".into(),
					value: "hello@atscale.xyz".into(),
					verify_method: VerifyMethod::Email,
				}]
				.into()
			),
		));

		// Test update identity data with invalid email
		assert_noop!(
			Identities::update_identity(
				Origin::signed(ALICE),
				None,
				Some(
					[IdentityFieldInput {
						name: "email".into(),
						value: "invalid_email".into(),
						verify_method: VerifyMethod::Email,
					}]
					.into()
				),
			),
			Error::<Runtime>::InvalidEmail
		);
	});
}

#[test]
fn update_identity_data_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[
				IdentityFieldInput {
					name: "email".into(),
					value: "foo@atscale.xyz".into(),
					verify_method: VerifyMethod::Email
				},
			].into(),
		));

		// Test add update identity data
		assert_ok!(Identities::update_identity_data(
			Origin::signed(ALICE),
			0,
			IdentityFieldInput {
				name: "email".into(),
				value: "hello@atscale.xyz".into(),
				verify_method: VerifyMethod::Email
			},
		));

		let identity = Identities::identities(&ALICE).unwrap();
		assert_eq!(identity.data.len(), 1);
		assert_eq!(identity.data[0].name, "email".as_bytes());
		assert_eq!(identity.data[0].value, "hello@atscale.xyz".as_bytes());

		assert_eq!(
			last_event(),
			Event::Identities(crate::Event::IdentityUpdated { account_id: ALICE })
		);
	});
}

#[test]
fn add_identity_data_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));

		// Test add valid identity data
		assert_ok!(Identities::add_identity_data(
			Origin::signed(ALICE),
			IdentityFieldInput {
				name: "email".into(),
				value: "hello@atscale.xyz".into(),
				verify_method: VerifyMethod::Email
			},
		));

		assert_eq!(
			last_event(),
			Event::Identities(crate::Event::IdentityUpdated { account_id: ALICE })
		);

		// Test add invalid domain.
		assert_noop!(
			Identities::add_identity_data(
				Origin::signed(ALICE),
				IdentityFieldInput {
					name: "domain".into(),
					value: "invalid_domain".into(),
					verify_method: VerifyMethod::Domain,
				},
			),
			Error::<Runtime>::InvalidDomain,
		);
	});
}

#[test]
fn add_identity_review_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_identity(
			Origin::signed(ALICE),
			"Alice".into(),
			IdentityType::Individual,
			[].into(),
		));

		assert_ok!(Identities::review_identity(Origin::signed(BOB), ALICE, "Good".into(),));

		let identity = Identities::identities(&ALICE).unwrap();

		assert_eq!(identity.reviews.len(), 1);
		assert_eq!(identity.reviews[0].reviewer, BOB);

		assert_noop!(
			Identities::review_identity(Origin::signed(BOB), ALICE, "Good".into(),),
			Error::<Runtime>::CanOnlyReviewOnce,
		);
	});
}

#[test]
fn create_evaluator_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_evaluator(
			Origin::signed(ALICE),
			"Alice".into(),
			"About Alice".into(),
			10,
		));
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 100);
		let evaluator = Identities::evaluators(&ALICE).unwrap();
		assert_eq!(evaluator.name, "Alice".as_bytes());
		assert_eq!(evaluator.about, "About Alice".as_bytes());
		assert_eq!(evaluator.rate, 10);
	});
}

#[test]
fn create_verify_request_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_evaluator(
			Origin::signed(ALICE),
			"Alice".into(),
			"About Alice".into(),
			10,
		));

		// Test update identity name
		assert_ok!(Identities::create_identity(
			Origin::signed(BOB),
			"BOB".into(),
			IdentityType::Individual,
			[IdentityFieldInput {
				name: "field_a".into(),
				value: "value_a".into(),
				verify_method: VerifyMethod::Evaluator,
			},]
			.into(),
		));

		assert_ok!(Identities::request_to_verify(Origin::signed(BOB), [0].into(), ALICE,));
		assert_eq!(Currencies::reserved_balance(CurrencyId::Native, &ALICE), 100);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 910);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &BOB), 990);

		let verify_requests = Identities::verify_data_requests(&ALICE).unwrap();
		assert_eq!(verify_requests.len(), 1);
	});
}

#[test]
fn verify_data_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Test update identity name
		assert_ok!(Identities::create_evaluator(
			Origin::signed(ALICE),
			"Alice".into(),
			"About Alice".into(),
			10,
		));

		// Test update identity name
		assert_ok!(Identities::create_identity(
			Origin::signed(BOB),
			"BOB".into(),
			IdentityType::Individual,
			[
				IdentityFieldInput {
					name: "field_a".into(),
					value: "value_a".into(),
					verify_method: VerifyMethod::Evaluator,
				},
				IdentityFieldInput {
					name: "field_b".into(),
					value: "value_b".into(),
					verify_method: VerifyMethod::Evaluator,
				},
				IdentityFieldInput {
					name: "field_c".into(),
					value: "value_c".into(),
					verify_method: VerifyMethod::Evaluator,
				},
			]
			.into(),
		));

		assert_ok!(Identities::request_to_verify(Origin::signed(BOB), [0, 2].into(), ALICE,));
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 920);
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &BOB), 980);

		assert_noop!(
			Identities::verify_data(Origin::signed(ALICE), BOB, [(1, true)].into(),),
			Error::<Runtime>::InvalidTranscript
		);

		assert_ok!(Identities::verify_data(
			Origin::signed(ALICE),
			BOB,
			[(0, true), (2, true)].into(),
		));

		let verify_requests = Identities::verify_data_requests(&ALICE).unwrap();
		assert_eq!(verify_requests.len(), 0);

		let identity = Identities::identities(&BOB).unwrap();

		assert_eq!(identity.data[0].is_verified, true);
		assert_eq!(identity.data[0].verify_by, Some(ALICE));
		assert_eq!(identity.data[2].is_verified, true);
		assert_eq!(identity.data[2].verify_by, Some(ALICE));
	});
}
