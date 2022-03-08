#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
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
			sp_std::if_std,
			sp_std::vec::Vec,
			traits::{
					Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, Randomness,
					WithdrawReasons,
			},
			transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
	}

	pub struct Identity<T: Config> {
		pub id: u128,
		pub name: Vec<u8>,
		pub domain: Vec<u8>,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn identities)]
	pub(super) type Identities<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Identity<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		IdentityCreated(T::Hash),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {

	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_identity(
			origin: OriginFor<T>,
			name: Vec<u8>,
			domain: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let identity = Identity::<T> {
				name,
				domain,
			};

			<Identities<T>>::insert(&sender, identify.clone());

			Self::deposit_event(Event::IdentityCreated());
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}
}
