//! # Dispute Resolution
//!
//! Dispute resolution is on-chain dispute resolving process to help resolve conflicts between
//! payment parties.
//!
//! ## Usage
//!
//! - `create_dispute` - Create an on-chain dispute to request refund. If the payee does not fight
//!   against the dispute, the refund will be execute after `DisputeFinalizingTime`.
//! - `fight_dispute` - Payee can fight against a dispute if make sure that invalid.
//! - `escalate_dispute` - If a party does not satisfied with the dispute result, they can escalate
//!   the dispute to more resolvers. Although there is no limit the escalate time, but the fee will
//!   increase follow the number of resolvers that involved to dispute case.
//! - propose_outcome - Selected resolvers make judgment after evaluation the argument.
//!
//! ## Events
//!
//! - DisputeCreated - A `payer` issue a dispute for a payment.
//! - DisputeFought - A `payee` fight against a dispute case.
//! - DisputeEscalated - `payer` or `payee` escalate a dispute to more resolvers.
//! - DisputeFinalized - A dispute is finalized after `DisputeFinalizingTime`. Once the dispute is
//!   resolved, there is no way to recover.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult, log, pallet_prelude::*, sp_runtime::traits::Hash,
		sp_std::vec::Vec,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use pallet_lrp::PaymentProtocol;
	use pallet_resolvers::ResolversNetwork;
	use pallet_timestamp::{self as timestamp};
	use primitives::CurrencyId;
	use scale_info::TypeInfo;
	use sp_io::offchain_index;
	use sp_runtime::{RuntimeDebug, SaturatedConversion};

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId<Self::Hash>>;
		type PaymentProtocol: PaymentProtocol<Self::Hash, Self::AccountId, BalanceOf<Self>>;
		type ResolversNetwork: ResolversNetwork<Self::AccountId, Self::Hash>;
		#[pallet::constant]
		type DisputeFinalizingTime: Get<MomentOf<Self>>;
		#[pallet::constant]
		type DisputeFee: Get<BalanceOf<Self>>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
	type HashOf<T> = <T as frame_system::Config>::Hash;
	type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;

	pub trait DisputeQuery<AccountId, Hash> {
		fn get_dispute_resolvers(payment_hash: Hash) -> Result<Vec<AccountId>, DispatchError>;
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Judgment {
		ReleaseFundToPayer,
		ReleaseFundToPayee,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum DisputeStatus {
		Finalizing,
		Evaluating,
		Resolved,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Argument<T: Config> {
		pub provider: AccountOf<T>,
		pub content_hash: HashOf<T>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Dispute<T: Config> {
		pub status: DisputeStatus,
		pub payment_hash: HashOf<T>,
		pub expired_at: MomentOf<T>,
		pub arguments: Vec<Argument<T>>,
		pub resolvers: Vec<AccountOf<T>>,
		pub fee: BalanceOf<T>,
		pub judgments: Vec<(AccountOf<T>, Judgment)>,
		pub outcome: Judgment,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	struct Transcript {
		release_to_payer: u64,
		release_to_payee: u64,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn finalizing_disputes)]
	pub(super) type FinalizingDisputes<T: Config> = StorageValue<_, Vec<HashOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn disputes)]
	pub(super) type Disputes<T: Config> = StorageMap<_, Twox64Concat, HashOf<T>, Dispute<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		DisputeCreated {
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			payment_hash: HashOf<T>,
		},
		DisputeFought {
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			payment_hash: HashOf<T>,
		},
		DisputeEscalated {
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			payment_hash: HashOf<T>,
		},
		DisputeResolved {
			payment_hash: HashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyId<T::Hash>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AccessDenied,
		DisputeNotAccepted,
		DisputeNotFound,
		ActionForOnlyFinalizingDispute,
		InsufficientBalance,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			if let Err(err) = Self::run_offchain_worker() {
				log::error!(
					target: "Dispute resolution offchain worker",
					"Fail to run offchain worker at block {:?}: {:?}",
					block_number,
					err,
				);
			} else {
				log::debug!(
					target: "Dispute resolution offchain worker",
					"offchain worker start at block: {:?} already done!",
					block_number,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_dispute(
			origin: OriginFor<T>,
			payment_hash: HashOf<T>,
			argument: Vec<u8>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			Self::_create_dispute(issuer, payment_hash, argument)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn fight_dispute(
			origin: OriginFor<T>,
			payment_hash: HashOf<T>,
			argument: Vec<u8>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			Self::_fight_dispute(issuer, payment_hash, argument)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn escalate_dispute(origin: OriginFor<T>, payment_hash: HashOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::_escalate_dispute(who, payment_hash)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn propose_outcome(
			origin: OriginFor<T>,
			payment_hash: HashOf<T>,
			judgement: Judgment,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::_propose_outcome(who, payment_hash, judgement)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn run_offchain_worker() -> DispatchResult {
			Self::_process_finalizing_disputes()?;
			Ok(())
		}

		fn _create_dispute(
			issuer: AccountOf<T>,
			payment_hash: HashOf<T>,
			argument: Vec<u8>,
		) -> DispatchResult {
			ensure!(T::PaymentProtocol::can_dispute(&payment_hash), <Error<T>>::DisputeNotAccepted);
			let (payer, payee, ..) = T::PaymentProtocol::get_payment(&payment_hash)?;
			ensure!(issuer == payer, <Error<T>>::AccessDenied);

			let fee = Self::_compute_dispute_fee(1);
			Self::_lock_resolvers_fee(&issuer, fee)?;

			let expired_at = <timestamp::Pallet<T>>::get() + T::DisputeFinalizingTime::get();

			let dispute = Dispute::<T> {
				payment_hash,
				expired_at,
				arguments: [Argument::<T> {
					provider: payer.clone(),
					content_hash: Self::_save_large_content(argument),
				}]
				.to_vec(),
				status: DisputeStatus::Finalizing,
				resolvers: [].to_vec(),
				judgments: [].to_vec(),
				fee,
				outcome: Judgment::ReleaseFundToPayer,
			};

			<Disputes<T>>::insert(&payment_hash, dispute);
			Self::_add_finalizing_dispute(payment_hash)?;

			Self::deposit_event(Event::DisputeCreated { payment_hash, payer, payee });

			Ok(())
		}

		fn _fight_dispute(
			who: AccountOf<T>,
			payment_hash: HashOf<T>,
			argument: Vec<u8>,
		) -> DispatchResult {
			let mut dispute = Self::disputes(&payment_hash).ok_or(<Error<T>>::DisputeNotFound)?;
			let (payer, payee, ..) = T::PaymentProtocol::get_payment(&payment_hash)?;

			let is_aggrieved_party =
				dispute.outcome == Judgment::ReleaseFundToPayee && who == payer ||
					dispute.outcome == Judgment::ReleaseFundToPayer && who == payee;

			ensure!(is_aggrieved_party, <Error<T>>::AccessDenied);
			ensure!(
				dispute.status == DisputeStatus::Finalizing,
				<Error<T>>::ActionForOnlyFinalizingDispute
			);

			dispute.status = DisputeStatus::Evaluating;
			dispute.arguments.push(Argument::<T> {
				provider: who.clone(),
				content_hash: Self::_save_large_content(argument),
			});

			// The number of resolvers will increases after each escalating round.
			let number_of_resolver = dispute.resolvers.len() + 1;

			Self::_lock_resolvers_fee(&who, dispute.fee)?;

			for _i in 0..number_of_resolver {
				let resolver = T::ResolversNetwork::get_resolver(
					payment_hash,
					dispute.resolvers.clone(),
				)?;
				dispute.resolvers.push(resolver);
			}

			Self::_remove_finalizing_dispute(&payment_hash)?;
			<Disputes<T>>::insert(&payment_hash, dispute);

			Self::deposit_event(Event::DisputeFought { payment_hash, payer, payee });

			Ok(())
		}

		fn _escalate_dispute(who: AccountOf<T>, payment_hash: HashOf<T>) -> DispatchResult {
			let (payer, payee, _, _) = T::PaymentProtocol::get_payment(&payment_hash)?;
			let mut dispute = Self::disputes(&payment_hash).ok_or(<Error<T>>::DisputeNotFound)?;

			ensure!(who == payer || who == payee, <Error<T>>::AccessDenied);
			ensure!(
				dispute.status == DisputeStatus::Finalizing,
				<Error<T>>::ActionForOnlyFinalizingDispute
			);

			let fee = Self::_compute_dispute_fee(dispute.resolvers.len() + 1);
			Self::_lock_resolvers_fee(&who, fee)?;
			dispute.fee += fee;

			if who == payer {
				dispute.outcome = Judgment::ReleaseFundToPayer;
			} else {
				dispute.outcome = Judgment::ReleaseFundToPayee;
			}

			dispute.status = DisputeStatus::Finalizing;
			<Disputes<T>>::insert(&payment_hash, dispute);

			Self::deposit_event(Event::DisputeEscalated { payment_hash, payer, payee });

			Ok(())
		}

		// Assigned resolvers propose their judgement after evaluation.
		fn _propose_outcome(
			who: AccountOf<T>,
			payment_hash: HashOf<T>,
			judment: Judgment,
		) -> DispatchResult {
			let mut dispute = Self::disputes(&payment_hash).ok_or(<Error<T>>::DisputeNotFound)?;

			// Ensure only selected resolver can propose for the outcome.
			ensure!(dispute.resolvers.contains(&who), <Error<T>>::AccessDenied);
			// Ensure selected resolver can give decision once.
			ensure!(!dispute.judgments.iter().any(|i| i.0 == who), <Error<T>>::AccessDenied);

			dispute.judgments.push((who, judment));

			// The dispute will be concluded if get enough judgements from resolvers.
			if dispute.resolvers.len() == dispute.judgments.len() {
				let mut transcript = Transcript { release_to_payer: 0, release_to_payee: 0 };

				for (_who, judgement) in dispute.judgments.iter() {
					match judgement {
						Judgment::ReleaseFundToPayee => transcript.release_to_payee += 1,
						Judgment::ReleaseFundToPayer => transcript.release_to_payer += 1,
					}
				}

				dispute.outcome = if transcript.release_to_payee > transcript.release_to_payer {
					Judgment::ReleaseFundToPayee
				} else {
					Judgment::ReleaseFundToPayer
				};

				dispute.status = DisputeStatus::Finalizing;
				Self::_add_finalizing_dispute(payment_hash)?;
			}

			<Disputes<T>>::insert(&payment_hash, dispute);

			Ok(())
		}

		fn _compute_dispute_fee(number_of_resolvers: usize) -> BalanceOf<T> {
			T::DisputeFee::get() * number_of_resolvers.saturated_into::<BalanceOf<T>>()
		}

		fn _lock_resolvers_fee(requestor: &AccountOf<T>, fee: BalanceOf<T>) -> DispatchResult {
			ensure!(
				T::Currency::free_balance(CurrencyId::Native, requestor) >= fee,
				<Error<T>>::InsufficientBalance,
			);
			T::Currency::reserve(CurrencyId::Native, requestor, fee)?;
			Ok(())
		}

		fn _release_resolvers_fee(who: &AccountOf<T>, fee: BalanceOf<T>) {
			T::Currency::unreserve(CurrencyId::Native, who, fee);
		}

		fn _distribute_resolvers_fee(
			who: &AccountOf<T>,
			resolvers: Vec<AccountOf<T>>,
		) -> DispatchResult {
			for resolver in resolvers {
				T::Currency::unreserve(CurrencyId::Native, who, T::DisputeFee::get());
				T::Currency::transfer(CurrencyId::Native, who, &resolver, T::DisputeFee::get())?;
			}
			Ok(())
		}

		fn _get_expired_time() -> MomentOf<T> {
			<timestamp::Pallet<T>>::get() + T::DisputeFinalizingTime::get()
		}

		fn _add_finalizing_dispute(hash: HashOf<T>) -> DispatchResult {
			<FinalizingDisputes<T>>::mutate(|hashes| hashes.push(hash));
			Ok(())
		}

		fn _remove_finalizing_dispute(hash: &HashOf<T>) -> DispatchResult {
			<FinalizingDisputes<T>>::mutate(|hashes| hashes.retain(|&h| h != *hash));
			Ok(())
		}

		fn _process_finalizing_disputes() -> DispatchResult {
			let hashes = <FinalizingDisputes<T>>::get();

			let mut resolved_disputes: Vec<T::Hash> = Vec::new();

			for hash in hashes.iter() {
				let mut dispute = Self::disputes(&hash).ok_or(<Error<T>>::DisputeNotFound)?;
				let now = <timestamp::Pallet<T>>::get();
				let (payer, payee, amount, currency_id) = T::PaymentProtocol::get_payment(hash)?;

				// If dispute is out of finalizing time, finalize it as the outcome.
				if now >= dispute.expired_at {
					match dispute.outcome {
						Judgment::ReleaseFundToPayee => {
							T::Currency::unreserve(currency_id, &payer, amount);
							T::Currency::transfer(
								currency_id,
								&payer,
								&payee,
								amount,
							)?;
							Self::_release_resolvers_fee(&payee, dispute.fee);
							Self::_distribute_resolvers_fee(&payer, dispute.resolvers.clone())?;
						},
						Judgment::ReleaseFundToPayer => {
							T::Currency::unreserve(currency_id, &payer, amount);
							Self::_release_resolvers_fee(&payer, dispute.fee);
							Self::_distribute_resolvers_fee(&payee, dispute.resolvers.clone())?;
						},
					}
					dispute.status = DisputeStatus::Resolved;

					<Disputes<T>>::insert(&hash, dispute);
					Self::deposit_event(Event::DisputeResolved {
						payment_hash: *hash,
						payer,
						payee,
						currency_id,
						amount,
					});

					resolved_disputes.push(*hash);
				} else {
					// The queue is sorted by time. If a dispute is in the waiting time, the rest of
					// the queue after the dispute is still in the waiting time.
					break
				}
			}

			// Remove resolved dispute from the queue.
			for hash in resolved_disputes.iter() {
				Self::_remove_finalizing_dispute(hash)?;
			}

			Ok(())
		}

		// Use offchain indexing to store large content in the offchain worker.
		fn _save_large_content(content: Vec<u8>) -> T::Hash {
			let content_hash = T::Hashing::hash_of(&content);
			offchain_index::set(&content_hash.encode(), &content);
			content_hash
		}
	}
}
