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
		sp_std::vec::Vec,
		traits::{Currency, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: ReservableCurrency<Self::AccountId>;
		#[pallet::constant]
		type BondingAmount: Get<BalanceOf<Self>>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
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
		StorageMap<_, Twox64Concat, T::AccountId, Vec<CurrencyIdOf<T>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CurrencyCreated {
			currency_id: CurrencyIdOf<T>,
			created_by: AccountOf<T>,
		},
		CurrencyRemoved {
			currency_id: CurrencyIdOf<T>,
			name: Vec<u8>,
			symbol: Vec<u8>,
			decimals: u8,
			removed_by: AccountOf<T>,
		},
		CurrencyAccepted {
			currency_id: CurrencyIdOf<T>,
			accepted_by: AccountOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		CurrencyExisted,
		CurrencyNotFound,
		NotCurrencyIssuer,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn create_currency(
			origin: OriginFor<T>,
			name: Vec<u8>,
			symbol: Vec<u8>,
			decimals: u8,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			let metadata = CurrencyMetadata::<T> { name, symbol, decimals, issuer: issuer.clone() };

			let currency_id = T::Hashing::hash_of(&metadata);

			ensure!(!<Currencies<T>>::contains_key(currency_id), <Error<T>>::CurrencyExisted);

			<Currencies<T>>::insert(&currency_id, metadata);
			T::Currency::reserve(&issuer, T::BondingAmount::get())?;

			Self::deposit_event(Event::CurrencyCreated { currency_id, created_by: issuer });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn remove_currency(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let metadata = Self::currencies(&currency_id).ok_or(<Error<T>>::CurrencyNotFound)?;

			ensure!(who == metadata.issuer, <Error<T>>::NotCurrencyIssuer);

			<Currencies<T>>::remove(&currency_id);
			T::Currency::unreserve(&who, T::BondingAmount::get());

			Self::deposit_event(Event::CurrencyRemoved {
				currency_id,
				name: metadata.name,
				symbol: metadata.symbol,
				decimals: metadata.decimals,
				removed_by: who,
			});

			Ok(())
		}

		#[pallet::weight(1_000 + T::DbWeight::get().writes(1))]
		pub fn accept_currency(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let merchant = ensure_signed(origin)?;

			ensure!(<Currencies<T>>::contains_key(currency_id), <Error<T>>::CurrencyNotFound);

			<AcceptedCurrencies<T>>::mutate(&merchant, |currency_ids| {
				currency_ids.push(currency_id.clone())
			});

			Self::deposit_event(Event::CurrencyAccepted { currency_id, accepted_by: merchant });

			Ok(())
		}
	}
}
