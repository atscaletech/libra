//! # Identities pallet
//! - [`Config`]
//! - [`Call`]
//! - [`Event`]
//! - [`Error`]
//!
//! # Overview
//! The identities pallet is a module that allows an individual/organization can create and manage
//! their own on-chain self-sovereign identity. There are 2 key factors of the modules:
//!
//! - Identity Data: The data provided by the identity owner. The data can be anything from email,
//!   and domain to legal data on the entity... The identity data in be used to do risk evaluation
//!   before making a transaction with the identity owner.
//!
//! - Identity Verification Service: 3rd services who deposit some tokens and take responsibility to
//!   verify specified fields of identity data to earn rewards. It can be an automation service such
//!   as email and domain verification or a KYC service.
//! # Usage
//! ## Identity Owner
//! - `create_identity`: create a new identity
//! - `update_identity`: update existed identity. This will replace the old identity with the new
//!   one.
//! - `update_identity_data`: update a data field of an existed identity
//! - `add_identity_data`: add a new data field to an existed identity
//! - `remove_identity`: remove an existed identity. The identity reviews will not be removed after
//!   this action.
//! - `request_to_verify`: request an evaluator to verify identity data
//! ## Identity Verify Services
//! - `create_evaluator`: bond native tokens to become evaluator.
//! - `verify_data`: verify data of a requested identity.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		sp_runtime::{traits::Hash, SaturatedConversion},
		sp_std::vec::Vec,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use primitives::{Credibility, CurrencyId};
	use scale_info::TypeInfo;
	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};
	use sp_io::offchain_index;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId<Self::Hash>>;
		/// The amount that an account need to bond to become an evaluator.
		#[pallet::constant]
		type EvaluatorBonding: Get<BalanceOf<Self>>;
		/// Initial credibility of an identity.
		#[pallet::constant]
		type InitialCredibility: Get<Credibility>;
		/// Max credibility of an identity.
		#[pallet::constant]
		type MaxCredibility: Get<Credibility>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

	pub trait IdentitiesManager<AccountId> {
		fn has_identity(account_id: &AccountId) -> bool;
		fn get_credibility(account_id: &AccountId) -> Result<Credibility, DispatchError>;
		fn increase_credibility(account_id: &AccountId, amount: Credibility) -> DispatchResult;
		fn decrease_credibility(account_id: &AccountId, amount: Credibility) -> DispatchResult;
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum VerifyMethod {
		Domain,
		Email,
		Evaluator,
		None,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum IdentityType {
		Individual,
		Organization,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub struct IdentityFieldInput {
		pub name: Vec<u8>,
		pub value: Vec<u8>,
		pub verify_method: VerifyMethod,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[scale_info(skip_type_params(T))]
	pub struct IdentityField<T: Config> {
		pub name: Vec<u8>,
		pub value: Vec<u8>,
		pub verify_method: VerifyMethod,
		pub is_verified: bool,
		pub verify_by: Option<AccountOf<T>>,
	}

	impl<T: Config> IdentityField<T> {
		pub fn from_identity_field_input(input: &IdentityFieldInput) -> IdentityField<T> {
			IdentityField::<T> {
				name: input.name.clone(),
				value: input.value.clone(),
				verify_method: input.verify_method.clone(),
				is_verified: false,
				verify_by: None,
			}
		}
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[scale_info(skip_type_params(T))]
	pub struct IdentityReview<T: Config> {
		pub reviewer: AccountOf<T>,
		pub content_digest: T::Hash,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Identity<T: Config> {
		pub name: Vec<u8>,
		pub identity_type: IdentityType,
		pub credibility: Credibility,
		pub data: Vec<IdentityField<T>>,
		pub reviews: Vec<IdentityReview<T>>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Evaluator<T: Config> {
		pub name: Vec<u8>,
		pub about: Vec<u8>,
		pub rate: BalanceOf<T>,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type VerifyDomainRequests<T: Config> =
		StorageValue<_, Vec<(Vec<u8>, AccountOf<T>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn verify_data_requests)]
	pub(super) type VerifyDataRequests<T: Config> =
		StorageMap<_, Twox64Concat, AccountOf<T>, Vec<(AccountOf<T>, Vec<u64>)>>;

	#[pallet::storage]
	#[pallet::getter(fn identities)]
	pub(super) type Identities<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, Identity<T>>;

	#[pallet::storage]
	#[pallet::getter(fn evaluators)]
	pub(super) type Evaluators<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, Evaluator<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The identity is created for the account.
		IdentityCreated { account_id: AccountOf<T> },
		/// The identity is updated.
		IdentityUpdated { account_id: AccountOf<T> },
		/// The identity is removed.
		IdentityRemoved { account_id: AccountOf<T> },
		/// The ownership of the domain is verified.
		DomainVerified { domain: Vec<u8>, owner: AccountOf<T> },
		/// The evaluator is created.
		EvaluatorCreated {
			account: AccountOf<T>,
			name: Vec<u8>,
			about: Vec<u8>,
			rate: BalanceOf<T>,
		},
		/// An account requests an evaluator to verify identity data of the account.
		VerifyDataRequestCreated {
			requestor: AccountOf<T>,
			positions: Vec<u64>,
			evaluator: AccountOf<T>,
		},
		/// An evaluator verify identity data of an account.
		DataVerified { account: AccountOf<T>, positions: Vec<u64>, evaluator: AccountOf<T> },
		/// An account create a review about another account.
		IdentityReviewAdded {
			account: AccountOf<T>,
			reviewer: AccountOf<T>,
			content_digest: T::Hash,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Identity of account is existed.
		IdentityExisted,
		/// There is no any identity of the account.
		IdentityNotFound,
		/// Evaluator data of the account is existed.
		EvaluatorExisted,
		/// The account is not an evaluator.
		EvaluatorNotFound,
		/// The account does not have appropriate access rights.
		AccessDenied,
		/// The submitted data is not a valid domain.
		InvalidDomain,
		/// The submitted data is not a valid email.
		InvalidEmail,
		/// There is no any data field matched with the condition.
		DataFieldNotFound,
		/// The evaluator is not requested to verify the identity.
		VerifyRequestNotFound,
		/// The transcript is not matched with the verify data request.
		InvalidTranscript,
		/// An account only can review other account once.
		CanOnlyReviewOnce,
	}

	// #[pallet::hooks]
	// impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
	// 	fn offchain_worker(block_number: T::BlockNumber) {
	// 		if let Err(err) = Self::run_offchain_worker() {
	// 			log::error!(
	// 				target: "Identities offchain worker",
	// 				"Fail to run offchain worker at block {:?}: {:?}",
	// 				block_number,
	// 				err,
	// 			);
	// 		} else {
	// 			log::debug!(
	// 				target: "Identities offchain worker",
	// 				"offchain worker start at block: {:?} already done!",
	// 				block_number,
	// 			);
	// 		}
	// 	}
	// }

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_identity(
			origin: OriginFor<T>,
			name: Vec<u8>,
			identity_type: IdentityType,
			data: Vec<IdentityFieldInput>,
		) -> DispatchResult {
			let requestor = ensure_signed(origin)?;
			Self::_create_identity(requestor, name, identity_type, data)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn update_identity(
			origin: OriginFor<T>,
			name: Option<Vec<u8>>,
			data: Option<Vec<IdentityFieldInput>>,
		) -> DispatchResult {
			let requestor = ensure_signed(origin)?;
			Self::_update_identity(requestor.clone(), name, data)?;
			Self::deposit_event(Event::IdentityUpdated { account_id: requestor });
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn update_identity_data(
			origin: OriginFor<T>,
			position: u64,
			data_field: IdentityFieldInput,
		) -> DispatchResult {
			let requestor = ensure_signed(origin)?;
			Self::_update_identity_data_field(
				requestor.clone(),
				position.try_into().unwrap(),
				data_field,
			)?;
			Self::deposit_event(Event::IdentityUpdated { account_id: requestor });
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn add_identity_data(
			origin: OriginFor<T>,
			data_field: IdentityFieldInput,
		) -> DispatchResult {
			let requestor = ensure_signed(origin)?;
			Self::_add_identity_data_field(requestor.clone(), data_field)?;
			Self::deposit_event(Event::IdentityUpdated { account_id: requestor });
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn remove_identity(origin: OriginFor<T>) -> DispatchResult {
			let requestor = ensure_signed(origin)?;
			// Only remove name and identity data
			Self::_update_identity(requestor.clone(), Some("".into()), Some([].into()))?;
			Self::deposit_event(Event::IdentityRemoved { account_id: requestor });
			Ok(())
		}

		#[pallet::weight(2_000)]
		pub fn review_identity(
			origin: OriginFor<T>,
			account: AccountOf<T>,
			content: Vec<u8>,
		) -> DispatchResult {
			let reviewer = ensure_signed(origin)?;
			Self::_add_identity_review(account, reviewer, content)?;
			Ok(())
		}

		// Request evaluator to review identity data.
		#[pallet::weight(1000)]
		pub fn create_evaluator(
			origin: OriginFor<T>,
			name: Vec<u8>,
			about: Vec<u8>,
			rate: BalanceOf<T>,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			Self::_create_evaluator(account, name, about, rate)?;
			Ok(())
		}

		// Request evaluator to review identity data.
		#[pallet::weight(1000)]
		pub fn request_to_verify(
			origin: OriginFor<T>,
			positions: Vec<u64>,
			evaluator: AccountOf<T>,
		) -> DispatchResult {
			let requestor = ensure_signed(origin)?;
			Self::_create_verify_data_request(requestor, positions, evaluator)?;
			Ok(())
		}

		// Verify data for customer.
		#[pallet::weight(1000)]
		pub fn verify_data(
			origin: OriginFor<T>,
			account: AccountOf<T>,
			transcript: Vec<(u64, bool)>,
		) -> DispatchResult {
			let evaluator = ensure_signed(origin)?;
			Self::_verify_data(evaluator, account, transcript)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn _create_identity(
			requestor: AccountOf<T>,
			name: Vec<u8>,
			identity_type: IdentityType,
			data: Vec<IdentityFieldInput>,
		) -> DispatchResult {
			ensure!(!<Identities<T>>::contains_key(&requestor), <Error<T>>::IdentityExisted);
			Self::_validate_data(data.clone())?;

			let identity = Identity {
				name,
				identity_type,
				credibility: T::InitialCredibility::get(),
				data: data
					.iter()
					.map(|input| IdentityField::from_identity_field_input(input))
					.collect(),
				reviews: [].to_vec(),
			};

			<Identities<T>>::insert(&requestor, identity);
			Self::deposit_event(Event::IdentityCreated { account_id: requestor });

			Ok(())
		}

		// WARNING: This will overwrite indentity data with new data.
		fn _update_identity(
			requestor: AccountOf<T>,
			name: Option<Vec<u8>>,
			data: Option<Vec<IdentityFieldInput>>,
		) -> DispatchResult {
			let mut identity = Self::identities(&requestor).ok_or(<Error<T>>::IdentityNotFound)?;

			if let Some(name) = name {
				identity.name = name;
			}

			if let Some(data) = data {
				Self::_validate_data(data.clone())?;
				identity.data = data
					.iter()
					.map(|input| IdentityField::from_identity_field_input(input))
					.collect();
			}

			<Identities<T>>::insert(&requestor, identity);

			Ok(())
		}

		fn _update_identity_data_field(
			requestor: AccountOf<T>,
			position: usize,
			data_field: IdentityFieldInput,
		) -> DispatchResult {
			Self::_validate_data_field(data_field.clone())?;
			let mut identity = Self::identities(&requestor).ok_or(<Error<T>>::IdentityNotFound)?;
			identity.data[position] = IdentityField::from_identity_field_input(&data_field);
			<Identities<T>>::insert(&requestor, identity);
			Ok(())
		}

		fn _add_identity_data_field(
			requestor: AccountOf<T>,
			data_field: IdentityFieldInput,
		) -> DispatchResult {
			Self::_validate_data_field(data_field.clone())?;
			let mut identity = Self::identities(&requestor).ok_or(<Error<T>>::IdentityNotFound)?;
			// TODO: considering not allow data field with the same name.
			identity.data.push(IdentityField::from_identity_field_input(&data_field));
			<Identities<T>>::insert(&requestor, identity);
			Ok(())
		}

		fn _add_identity_review(
			account: AccountOf<T>,
			reviewer: AccountOf<T>,
			content: Vec<u8>,
		) -> DispatchResult {
			let mut identity = Self::identities(&account).ok_or(<Error<T>>::IdentityNotFound)?;

			let has_review = identity.reviews.iter().any(|r| r.reviewer == reviewer);
			ensure!(!has_review, <Error<T>>::CanOnlyReviewOnce);

			let content_digest = T::Hashing::hash_of(&content);
			offchain_index::set(&content_digest.encode(), &content);

			let review = IdentityReview { reviewer, content_digest };
			identity.reviews.push(review.clone());

			<Identities<T>>::insert(&account, identity);

			Self::deposit_event(Event::IdentityReviewAdded {
				account,
				reviewer: review.reviewer,
				content_digest: review.content_digest,
			});
			Ok(())
		}

		fn _create_evaluator(
			account: AccountOf<T>,
			name: Vec<u8>,
			about: Vec<u8>,
			rate: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(!<Evaluators<T>>::contains_key(&account), <Error<T>>::EvaluatorExisted);
			T::Currency::reserve(CurrencyId::Native, &account, T::EvaluatorBonding::get())?;
			let evaluator = Evaluator::<T> { name, about, rate };
			<Evaluators<T>>::insert(&account, evaluator.clone());
			Self::deposit_event(Event::EvaluatorCreated {
				account,
				name: evaluator.name,
				about: evaluator.about,
				rate: evaluator.rate,
			});
			Ok(())
		}

		fn _create_verify_data_request(
			requestor: AccountOf<T>,
			positions: Vec<u64>,
			evaluator_address: AccountOf<T>,
		) -> DispatchResult {
			let evaluator =
				Self::evaluators(&evaluator_address).ok_or(<Error<T>>::EvaluatorNotFound)?;

			let cost = evaluator.rate * positions.len().saturated_into::<BalanceOf<T>>();

			T::Currency::transfer(CurrencyId::Native, &requestor, &evaluator_address, cost)?;

			let verify_requests = Self::verify_data_requests(&evaluator_address);

			if let Some(mut verify_requests) = verify_requests {
				verify_requests.push((requestor.clone(), positions.clone()));
				<VerifyDataRequests<T>>::insert(&evaluator_address, verify_requests);
			} else {
				<VerifyDataRequests<T>>::insert(
					&evaluator_address,
					[(requestor.clone(), positions.clone())].to_vec(),
				);
			}

			Self::deposit_event(Event::VerifyDataRequestCreated {
				requestor,
				positions,
				evaluator: evaluator_address,
			});

			Ok(())
		}

		fn _verify_data(
			evaluator: AccountOf<T>,
			account: AccountOf<T>,
			transcript: Vec<(u64, bool)>,
		) -> DispatchResult {
			let mut identity = Self::identities(&account).ok_or(<Error<T>>::IdentityNotFound)?;
			let mut verify_requests =
				Self::verify_data_requests(&evaluator).ok_or(<Error<T>>::VerifyRequestNotFound)?;

			let request = verify_requests.iter().find(|r| r.0 == account);

			if let Some(request) = request {
				let transcript_pos: Vec<u64> = transcript.iter().map(|item| item.0).collect();

				ensure!(transcript_pos == request.1, <Error<T>>::InvalidTranscript);

				for (position, is_valid) in transcript {
					if is_valid {
						identity.data[position as usize].is_verified = true;
						identity.data[position as usize].verify_by = Some(evaluator.clone());
					}
				}

				verify_requests.retain(|r| r.0 != account);

				<VerifyDataRequests<T>>::insert(&evaluator, verify_requests);
				<Identities<T>>::insert(&account, identity);
				return Ok(())
			}

			Err(<Error<T>>::VerifyRequestNotFound.into())
		}

		fn _validate_data(data: Vec<IdentityFieldInput>) -> DispatchResult {
			for field in data {
				Self::_validate_data_field(field)?;
			}
			Ok(())
		}

		fn _validate_data_field(data_field: IdentityFieldInput) -> DispatchResult {
			match data_field.verify_method {
				VerifyMethod::Domain => {
					ensure!(Self::_is_valid_domain(&data_field.value), <Error<T>>::InvalidDomain);
				},
				VerifyMethod::Email => {
					ensure!(Self::_is_valid_email(data_field.value), <Error<T>>::InvalidEmail);
				},
				_ => (),
			}

			Ok(())
		}

		fn _is_valid_domain(value: &[u8]) -> bool {
			if value.len() < 5 {
				return false
			}

			let dot = ".".as_bytes();

			let mut dot_indexes: Vec<usize> = Vec::new();
			let arr = value.windows(dot.len());
			let len = arr.len();
			for (index, item) in arr.enumerate() {
				// domain starts with .
				if index == 0 && item == dot {
					return false
				}
				// domain ends with .
				if index == len - 1 && item == dot {
					return false
				}
				if item == dot {
					if Some(index - 1) == dot_indexes.last().cloned() {
						return false
					}
					dot_indexes.push(index);
				}
			}

			// domain does not contain .
			if dot_indexes.is_empty() {
				return false
			}

			true
		}

		fn _is_valid_email(value: Vec<u8>) -> bool {
			if value.len() < 5 {
				return false
			}

			const AT_SYMBOL: u8 = 64;

			let at = value.iter().position(|c| c == &AT_SYMBOL);
			if let Some(at) = at {
				let (start, end) = value.split_at(at + 1);
				if !start.starts_with(&[AT_SYMBOL]) && Self::_is_valid_domain(end) {
					return true
				}
			}

			false
		}

		fn _verify_domain(domain: Vec<u8>, owner: AccountOf<T>) -> DispatchResult {
			let mut identity = Self::identities(&owner).ok_or(<Error<T>>::IdentityNotFound)?;

			let position = identity.data.iter().position(|field| {
				field.verify_method == VerifyMethod::Domain && field.value == domain
			});

			if let Some(position) = position {
				identity.data[position].is_verified = true;
				identity.data[position].verify_by = None;

				Self::deposit_event(Event::DomainVerified { domain, owner });

				Ok(())
			} else {
				Err(<Error<T>>::DataFieldNotFound.into())
			}
		}
	}

	impl<T: Config> IdentitiesManager<T::AccountId> for Pallet<T> {
		fn has_identity(account_id: &T::AccountId) -> bool {
			<Identities<T>>::contains_key(account_id)
		}

		fn get_credibility(account_id: &T::AccountId) -> Result<Credibility, DispatchError> {
			let identity = Self::identities(&account_id).ok_or(<Error<T>>::IdentityNotFound)?;
			Ok(identity.credibility)
		}

		/// Increase the credibility for the identity of the identity made a good behavior.
		fn increase_credibility(account_id: &T::AccountId, amount: Credibility) -> DispatchResult {
			let mut identity = Self::identities(&account_id)
				.ok_or(<Error<T>>::IdentityNotFound)?;

			if identity.credibility + amount >= T::MaxCredibility::get() {
				identity.credibility = T::MaxCredibility::get();
			} else {
				identity.credibility += amount;
			}

			<Identities<T>>::insert(account_id, identity);

			Ok(())
		}

		/// Increase the credibility for the identity of the identity made a bad behavior.
		fn decrease_credibility(account_id: &T::AccountId, amount: Credibility) -> DispatchResult {
			let mut identity = Self::identities(&account_id).ok_or(<Error<T>>::IdentityNotFound)?;
			identity.credibility -= amount;
			<Identities<T>>::insert(account_id, identity);
			Ok(())
		}
	}
}
