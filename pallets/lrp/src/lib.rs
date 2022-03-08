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
		sp_std::vec::Vec, traits::LockIdentifier,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiLockableCurrency};
	use orml_utilities::OffchainErr;
	use pallet_timestamp::{self as timestamp};
	use scale_info::TypeInfo;

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiLockableCurrency<Self::AccountId>;
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
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn latest_payment_id)]
	pub(super) type LatestPaymentId<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn payments)]
	pub(super) type Payments<T: Config> = StorageMap<_, Twox64Concat, PaymentHashOf<T>, Payment<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PaymentCreated(PaymentHashOf<T>),
		PaymentAccepted(PaymentHashOf<T>),
		PaymentRejected(PaymentHashOf<T>),
		PaymentExpired(PaymentHashOf<T>),
		PaymentFullFilled(PaymentHashOf<T>),
		PaymentCancelled(PaymentHashOf<T>),
		PaymentDisputed(PaymentHashOf<T>),
		PaymentCompleted(PaymentHashOf<T>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		Overflow,
		InsufficientBalance,
		PaymentNotFound,
		AccessDenied,
		InvalidStatusChange,
		PaymentUpdateFailed,
		TransferCurrencyFailed,
		RemoveLockFailed,
		ExtendLockFailed,
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

	pub const PAYMENT_LOCK_ID: LockIdentifier = *b"paidlock";
	pub const MAX_PENDING_TIME: u32 = 172800000;
	pub const MAX_FULL_FILL_TIME: u32 = 2592000000;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_payment(
			origin: OriginFor<T>,
			payee: AccountOf<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyIdOf<T>,
			description: Vec<u8>,
			receipt_hash: T::Hash,
		) -> DispatchResult {
			let payer = ensure_signed(origin)?;
			Self::do_create_payment(payer, payee, amount, currency_id, description, receipt_hash)?;
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
		fn run_offchain_worker() -> Result<(), OffchainErr> {
			Ok(())
		}

		fn do_create_payment(
			payer: AccountOf<T>,
			payee: AccountOf<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyIdOf<T>,
			description: Vec<u8>,
			receipt_hash: T::Hash,
		) -> DispatchResult {
			let id = Self::latest_payment_id().checked_add(1).ok_or(<Error<T>>::Overflow)?;

			ensure!(
				T::Currency::free_balance(currency_id.clone(), &payer) >= amount,
				<Error<T>>::InsufficientBalance,
			);

			T::Currency::extend_lock(PAYMENT_LOCK_ID, currency_id.clone(), &payer, amount.clone())?;

			let now = <timestamp::Pallet<T>>::get();

			let payment = Payment::<T> {
				id,
				payer,
				payee,
				amount,
				currency_id,
				description,
				receipt_hash,
				created_at: now,
				updated_at: now,
				status: PaymentStatus::Pending,
			};

			let payment_hash = T::Hashing::hash_of(&payment);

			<Payments<T>>::insert(&payment_hash, payment);
			<LatestPaymentId<T>>::put(id);

			Self::deposit_event(Event::PaymentCreated(payment_hash));

			Ok(())
		}

		fn do_update_payment(
			payment_hash: PaymentHashOf<T>,
			status: PaymentStatus,
		) -> DispatchResult {
			let mut payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			let now = <timestamp::Pallet<T>>::get();

			payment.updated_at = now;
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

			Self::do_update_payment(payment_hash, PaymentStatus::Accepted)?;
			Self::deposit_event(Event::PaymentAccepted(payment_hash));

			Ok(())
		}

		fn do_reject_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			ensure!(sender == payment.payee, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::Pending, <Error<T>>::InvalidStatusChange);

			T::Currency::remove_lock(PAYMENT_LOCK_ID, payment.currency_id.clone(), &payment.payer)?;

			Self::do_update_payment(payment_hash, PaymentStatus::Rejected)?;
			Self::deposit_event(Event::PaymentRejected(payment_hash));
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

			T::Currency::remove_lock(PAYMENT_LOCK_ID, payment.currency_id.clone(), &payment.payer)?;

			Self::do_update_payment(payment_hash, PaymentStatus::Cancelled)?;
			Self::deposit_event(Event::PaymentCancelled(payment_hash));

			Ok(())
		}

		fn do_full_fill_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;

			ensure!(sender == payment.payee, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::Accepted, <Error<T>>::InvalidStatusChange);

			Self::do_update_payment(payment_hash, PaymentStatus::FullFilled)?;
			Self::deposit_event(Event::PaymentFullFilled(payment_hash));

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

			Self::do_update_payment(payment_hash, PaymentStatus::Disputed)?;
			Self::deposit_event(Event::PaymentFullFilled(payment_hash));

			Ok(())
		}

		fn do_complete_payment(
			sender: AccountOf<T>,
			payment_hash: PaymentHashOf<T>,
		) -> DispatchResult {
			let payment = Self::payments(&payment_hash).ok_or(<Error<T>>::PaymentNotFound)?;
			ensure!(sender == payment.payee, <Error<T>>::AccessDenied);
			ensure!(payment.status == PaymentStatus::FullFilled, <Error<T>>::InvalidStatusChange);

			T::Currency::remove_lock(PAYMENT_LOCK_ID, payment.currency_id.clone(), &payment.payer)?;

			T::Currency::transfer(
				payment.currency_id.clone(),
				&payment.payer,
				&payment.payee,
				payment.amount.clone(),
			)?;

			Self::do_update_payment(payment_hash, PaymentStatus::Completed)?;
			Self::deposit_event(Event::PaymentCompleted(payment_hash));

			Ok(())
		}
	}
}
