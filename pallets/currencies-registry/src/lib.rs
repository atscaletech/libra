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
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use primitives::{ CurrencyId };
	use scale_info::TypeInfo;

	pub trait CurrenciesManager<AccountId, Hash> {
		fn is_currency_accepted(merchant: &AccountId, currency_id: &CurrencyId<Hash>) -> bool;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
	type CurrencyHashOf<T> = <T as frame_system::Config>::Hash;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiReservableCurrency<
			Self::AccountId,
			CurrencyId = CurrencyId<Self::Hash>,
		>;
		#[pallet::constant]
		type BondingAmount: Get<BalanceOf<Self>>;
	}

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
		StorageMap<_, Twox64Concat, T::AccountId, Vec<CurrencyHashOf<T>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CurrencyCreated {
			currency_hash: CurrencyHashOf<T>,
			created_by: AccountOf<T>,
		},
		CurrencyRemoved {
			currency_hash: CurrencyHashOf<T>,
			name: Vec<u8>,
			symbol: Vec<u8>,
			decimals: u8,
			removed_by: AccountOf<T>,
		},
		CurrencyAccepted {
			currency_hash: CurrencyHashOf<T>,
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

			let currency_hash = T::Hashing::hash_of(&metadata);

			ensure!(!<Currencies<T>>::contains_key(currency_hash), <Error<T>>::CurrencyExisted);

			<Currencies<T>>::insert(&currency_hash, metadata);
			T::Currency::reserve(CurrencyId::Native.into(), &issuer, T::BondingAmount::get())?;

			Self::deposit_event(Event::CurrencyCreated { currency_hash, created_by: issuer });

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn remove_currency(
			origin: OriginFor<T>,
			currency_hash: CurrencyHashOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let metadata = Self::currencies(&currency_hash).ok_or(<Error<T>>::CurrencyNotFound)?;

			ensure!(who == metadata.issuer, <Error<T>>::NotCurrencyIssuer);

			<Currencies<T>>::remove(&currency_hash);
			T::Currency::unreserve(<CurrencyId<T::Hash>>::Native, &who, T::BondingAmount::get());

			Self::deposit_event(Event::CurrencyRemoved {
				currency_hash,
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
			currency_hash: CurrencyHashOf<T>,
		) -> DispatchResult {
			let merchant = ensure_signed(origin)?;

			ensure!(<Currencies<T>>::contains_key(currency_hash), <Error<T>>::CurrencyNotFound);

			<AcceptedCurrencies<T>>::mutate(&merchant, |currency_ids| {
				currency_ids.push(currency_hash.clone())
			});

			Self::deposit_event(Event::CurrencyAccepted { currency_hash, accepted_by: merchant });

			Ok(())
		}
	}

	impl<T: Config> CurrenciesManager<T::AccountId, T::Hash> for Pallet<T> {
		fn is_currency_accepted(merchant: &T::AccountId, currency_id: &CurrencyId<T::Hash>) -> bool {
			match currency_id {
				CurrencyId::<T::Hash>::Native => return true,
				CurrencyId::<T::Hash>::Registered(hash) => {
					let accepted_currencies = Self::accepted_currencies(merchant);
					return accepted_currencies.contains(&hash);
				},
			}
		}
	}
}
