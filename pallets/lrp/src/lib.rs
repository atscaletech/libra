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
	use pallet_timestamp::{self as timestamp};
	use scale_info::TypeInfo;
	use sp_io::offchain_index;

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiReservableCurrency<Self::AccountId>;
		#[pallet::constant]
		type PendingPaymentWaitingTime: Get<MomentOf<Self>>;
		#[pallet::constant]
		type FullFilledPaymentWaitingTime: Get<MomentOf<Self>>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
	type CurrencyIdOf<T> = <<T as Config>::Currency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;
	type PaymentHashOf<T> = <T as frame_system::Config>::Hash;
	type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum PaymentStatus {
		Pending,
		Accepted,
		Rejected,
		Expired,
		FullFilled,
		Disputed,
		Cancelled,
		Completed,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Payment<T: Config> {
		pub id: u128,
		pub payer: AccountOf<T>,
		pub payee: AccountOf<T>,
		pub amount: BalanceOf<T>,
		pub currency_id: CurrencyIdOf<T>,
		pub description: Vec<u8>,
		pub status: PaymentStatus,
		pub receipt_hash: T::Hash,
		pub created_at: MomentOf<T>,
		pub updated_at: MomentOf<T>,
		pub updated_by: AccountOf<T>,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type LatestPaymentId<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn payments)]
	pub(super) type Payments<T: Config> = StorageMap<_, Twox64Concat, PaymentHashOf<T>, Payment<T>>;

	#[pallet::storage]
	pub(super) type PendingPaymentHashes<T: Config> =
		StorageValue<_, Vec<PaymentHashOf<T>>, ValueQuery>;

	#[pallet::storage]
	pub(super) type FullFilledPaymentHashes<T: Config> =
		StorageValue<_, Vec<PaymentHashOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn payments_owned)]
	pub(super) type PaymentsOwned<T: Config> =
		StorageMap<_, Twox64Concat, AccountOf<T>, Vec<PaymentHashOf<T>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PaymentCreated {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentAccepted {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentRejected {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentExpired {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentFullFilled {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentCancelled {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentDisputed {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		PaymentCompleted {
			payment_hash: PaymentHashOf<T>,
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		Overflow,
		InsufficientBalance,
		PaymentNotFound,
		AccessDenied,
		InvalidStatusChange,
		PaymentNonexpired,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			if let Err(err) = Self::run_offchain_worker() {
				log::error!(
					target: "LRP protocol offchain worker",
					"Fail to run offchain worker at block {:?}: {:?}",
					block_number,
					err,
				);
			} else {
				log::debug!(
					target: "LRP protocol offchain worker",
					"offchain worker start at block: {:?} already done!",
					block_number,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_payment(
			origin: OriginFor<T>,
			payee: AccountOf<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyIdOf<T>,
			description: Vec<u8>,
			receipt: Vec<u8>,
		) -> DispatchResult {
			let payer = ensure_signed(origin)?;
			Self::do_create_payment(payer, payee, amount, currency_id, description, receipt)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn accept_payment(
			origin: OriginFor<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_accept_payment(sender, payment_hash)?;

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn reject_payment(
			origin: OriginFor<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_reject_payment(sender, payment_hash)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn cancel_payment(
			origin: OriginFor<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_cancel_payment(sender, payment_hash)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn dispute_payment(
			origin: OriginFor<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_dispute_payment(sender, payment_hash)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn full_fill_payment(
			origin: OriginFor<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_full_fill_payment(sender, payment_hash)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn complete_payment(
			origin: OriginFor<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_complete_payment(sender, payment_hash)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn run_offchain_worker() -> DispatchResult {
			Self::evaluate_pending_payments()?;
			Self::evaluate_full_filled_payments()?;

			Ok(())
		}

		fn evaluate_pending_payments() -> DispatchResult {
			let pending_payment_hashes = <PendingPaymentHashes<T>>::get();
			if pending_payment_hashes.len() == 0 {
				return Ok(())
			}

			for payment_hash in pending_payment_hashes.iter() {
				Self::do_expire_payment(payment_hash.clone())?;
			}

			Ok(())
		}

		fn evaluate_full_filled_payments() -> DispatchResult {
			let full_filled_payment_hashes = <FullFilledPaymentHashes<T>>::get();

			if full_filled_payment_hashes.len() == 0 {
				return Ok(())
			}

			for payment_hash in full_filled_payment_hashes.iter() {
				Self::do_auto_complete_full_filled_payments(payment_hash.clone())?;
			}

			Ok(())
		}

		fn do_create_payment(
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyIdOf<T>,
			description: Vec<u8>,
			receipt: Vec<u8>,
		) -> DispatchResult {
			let id = <LatestPaymentId<T>>::get().checked_add(1).ok_or(<Error<T>>::Overflow)?;

			ensure!(
				T::Currency::free_balance(currency_id.clone(), &payer) >= amount,
				<Error<T>>::InsufficientBalance,
			);

			T::Currency::reserve(currency_id.clone(), &payer, amount.clone())?;

			let now = <timestamp::Pallet<T>>::get();
			let receipt_hash = T::Hashing::hash_of(&receipt);

			offchain_index::set(&receipt_hash.encode(), &receipt);

			let payment = Payment::<T> {
				id,
				payer: payer.clone(),
				payee: payee.clone(),
				amount,
				currency_id: currency_id.clone(),
				description,
				receipt_hash,
				created_at: now,
				updated_at: now,
				updated_by: payer.clone(),
				status: PaymentStatus::Pending,
			};

			let payment_hash = T::Hashing::hash_of(&payment);

			<Payments<T>>::insert(&payment_hash, payment);
			<PaymentsOwned<T>>::mutate(&payer, |payment_hashes| {
				payment_hashes.push(payment_hash.clone())
			});
			<LatestPaymentId<T>>::put(id);
			<PendingPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.push(payment_hash.clone())
			});

			Self::deposit_event(Event::PaymentCreated {
				payment_hash,
				payer,
				payee,
				currency_id,
				amount,
			});

			Ok(())
		}

		fn do_update_payment(
			updated_by: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
			status: PaymentStatus,
		) -> DispatchResult {
			let mut payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			let now = <timestamp::Pallet<T>>::get();

			payment.updated_at = now;
			payment.updated_by = updated_by;
			payment.status = status;

			<Payments<T>>::insert(&payment_hash, payment);
			Ok(())
		}

		fn do_accept_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			ensure!(sender == payment.payee, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::Pending, <Error<T>>::InvalidStatusChange);

			Self::do_update_payment(sender, payment_hash, PaymentStatus::Accepted)?;

			<PendingPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.retain(|&hash| hash != payment_hash)
			});

			Self::deposit_event(Event::PaymentAccepted {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_reject_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			ensure!(sender == payment.payee, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::Pending, <Error<T>>::InvalidStatusChange);

			T::Currency::unreserve(
				payment.currency_id.clone(),
				&payment.payer,
				payment.amount.clone(),
			);

			Self::do_update_payment(sender, payment_hash, PaymentStatus::Rejected)?;

			<PendingPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.retain(|&hash| hash != payment_hash)
			});

			Self::deposit_event(Event::PaymentRejected {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_expire_payment(payment_hash: PaymentHashOf<T>) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			ensure!(payment.status == PaymentStatus::Pending, <Error<T>>::InvalidStatusChange);

			let now = <timestamp::Pallet<T>>::get();
			let expired_time = payment.updated_at + T::PendingPaymentWaitingTime::get();

			if expired_time < now {
				return Ok(())
			}

			T::Currency::unreserve(
				payment.currency_id.clone(),
				&payment.payer,
				payment.amount.clone(),
			);

			Self::do_update_payment(payment.updated_by, payment_hash, PaymentStatus::Expired)?;

			<PendingPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.retain(|&hash| hash != payment_hash)
			});

			Self::deposit_event(Event::PaymentExpired {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_cancel_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			match payment.status {
				PaymentStatus::Pending =>
					ensure!(sender == payment.payer, <Error<T>>::AccessDenied),
				PaymentStatus::Accepted =>
					ensure!(sender == payment.payee, <Error<T>>::AccessDenied),
				_ => return Err(<Error<T>>::InvalidStatusChange.into()),
			}

			T::Currency::unreserve(
				payment.currency_id.clone(),
				&payment.payer,
				payment.amount.clone(),
			);

			Self::do_update_payment(sender, payment_hash, PaymentStatus::Cancelled)?;

			Self::deposit_event(Event::PaymentCancelled {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_full_fill_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			ensure!(sender == payment.payee, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::Accepted, <Error<T>>::InvalidStatusChange);

			Self::do_update_payment(sender, payment_hash, PaymentStatus::FullFilled)?;

			<FullFilledPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.push(payment_hash.clone())
			});

			Self::deposit_event(Event::PaymentFullFilled {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_auto_complete_full_filled_payments(payment_hash: PaymentHashOf<T>) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;
			let now = <timestamp::Pallet<T>>::get();

			let expired_time = payment.updated_at + T::FullFilledPaymentWaitingTime::get();

			if expired_time < now {
				return Ok(())
			}

			T::Currency::unreserve(
				payment.currency_id.clone(),
				&payment.payer,
				payment.amount.clone(),
			);

			T::Currency::transfer(
				payment.currency_id.clone(),
				&payment.payer,
				&payment.payee,
				payment.amount.clone(),
			)?;

			Self::do_update_payment(payment.updated_by, payment_hash, PaymentStatus::Completed)?;

			<FullFilledPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.retain(|&hash| hash != payment_hash)
			});

			Self::deposit_event(Event::PaymentCompleted {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_dispute_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			match payment.status {
				PaymentStatus::Accepted => ensure!(
					sender == payment.payer || sender == payment.payee,
					<Error<T>>::AccessDenied
				),
				PaymentStatus::FullFilled => ensure!(
					sender == payment.payer || sender == payment.payee,
					<Error<T>>::AccessDenied
				),
				_ => return Err(<Error<T>>::InvalidStatusChange.into()),
			}

			Self::do_update_payment(sender, payment_hash, PaymentStatus::Disputed)?;
			Self::deposit_event(Event::PaymentDisputed {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}

		fn do_complete_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;
			ensure!(sender == payment.payer, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::FullFilled, <Error<T>>::InvalidStatusChange);

			T::Currency::unreserve(
				payment.currency_id.clone(),
				&payment.payer,
				payment.amount.clone(),
			);

			T::Currency::transfer(
				payment.currency_id.clone(),
				&payment.payer,
				&payment.payee,
				payment.amount.clone(),
			)?;

			Self::do_update_payment(sender, payment_hash, PaymentStatus::Completed)?;

			<FullFilledPaymentHashes<T>>::mutate(|payment_hashes| {
				payment_hashes.retain(|&hash| hash != payment_hash)
			});

			Self::deposit_event(Event::PaymentCompleted {
				payment_hash,
				payer: payment.payer,
				payee: payment.payee,
				currency_id: payment.currency_id,
				amount: payment.amount,
			});

			Ok(())
		}
	}
}
