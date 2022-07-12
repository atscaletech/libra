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
		dispatch::DispatchResult, pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use primitives::CurrencyId;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId<Self::Hash>>;
		#[pallet::constant]
		type NativeTokenDeposit: Get<BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FaucetAccepted {
			account: AccountOf<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyId<T::Hash>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		FaucetRejected,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn faucet(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			currency_id: CurrencyId<T::Hash>,
		) -> DispatchResult {
			let requestor = ensure_signed(origin)?;

			if currency_id != CurrencyId::Native {
				T::Currency::deposit(
					CurrencyId::Native,
					&requestor,
					T::NativeTokenDeposit::get(),
				)?;
			}

			T::Currency::deposit(currency_id.clone(), &requestor, amount.clone())?;

			Self::deposit_event(Event::FaucetAccepted { account: requestor, amount, currency_id });

			Ok(())
		}
	}
}
