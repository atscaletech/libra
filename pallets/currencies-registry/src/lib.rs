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
		dispatch::DispatchResult,
		pallet_prelude::*,
		sp_runtime::traits::Hash,
		sp_std::{if_std, vec::Vec},
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_core::H256;

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type CurrencyIdOf<T> = <T as frame_system::Config>::Hash;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct CurrencyMetadata<T: Config> {
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub decimals: u8,
		pub issuer: AccountOf<T>,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn currencies)]
	pub(super) type Currencies<T: Config> =
		StorageMap<_, Twox64Concat, T::Hash, CurrencyMetadata<T>>;

	#[pallet::storage]
	#[pallet::getter(fn accepted_currencies)]
	pub(super) type AcceptedCurrencies<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<CurrencyIdOf<T>>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CurrencyCreated(CurrencyIdOf<T>),
		CurrencyAccepted(CurrencyIdOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		CurrencyExisted,
		CurrencyNotExist,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_currency(
			origin: OriginFor<T>,
			name: Vec<u8>,
			symbol: Vec<u8>,
			decimals: u8,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			let currency_metadata = CurrencyMetadata::<T> { name, symbol, decimals, issuer };

			let currency_id = T::Hashing::hash_of(&currency_metadata);

			ensure!(!<Currencies<T>>::contains_key(currency_id), <Error<T>>::CurrencyExisted,);

			<Currencies<T>>::insert(&currency_id, currency_metadata);

			Self::deposit_event(Event::CurrencyCreated(currency_id));
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn accept_currency(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let merchant = ensure_signed(origin)?;

			Self::currencies(&currency_id).ok_or(<Error<T>>::CurrencyNotExist)?;

			Self::deposit_event(Event::CurrencyAccepted(currency_id));
			Ok(())
		}
	}
}
