//! # Resolvers Network
//!
//! Resolvers network is a decentralized arbitrators network that allows an arbitrator to stake some
//! native tokens to join and resolve payment conflicts between parties to receive the fee from
//! dispute parties. A resolver can just stake an amount that meets the minimum requirements and run
//! a community crowd loan to get enough delegations to become an active resolver. The delegators
//! will share the rewards with the resolver.
//!
//! ## Functions
//!
//! - `join_resolver_networks` - Apply to become resolver. If the `self_stake` amount reach the
//!   `ActivationStakeAmount`, the resolver will be active. Other wise, it will be remain
//!   `Candidacy` status.
//! - `delegate` - Delegate some native token to a resolver. If the `total_stake` (`self_stake` +
//!   `delegations`) reach the `ActivationStakeAmount`, the resolver will be active.
//! - `undelegate` - Remove delegation from a resolver. If the `total_stake` drop bellow the
//!   `ActivationStakeAmount`, the resolver will be inactive and become candidacy resolver.
//! - `resign` - Leave the resolver position and get back the deposited tokens. The delegations will
//!   be refunded after `UndelegateTime`.
//!
//! ## Traits
//!
//! ResolverNetwork
//! - get_resolver - Get a random resolver from resolvers network.
//! - increase_credibility - Increase a resolver's credibility
//! A resolver can gain credibility by resolving a dispute with correct judgment. The credibility
//! cannot exceed the `MaxCredibility`.
//! - reduce_credibility - Reduce a resolver's credibility
//! A resolver can lose credibility if they made a mistake in dispute resolving process. If the
//! credibility of a resolver falls below the `MinCredibility`, it will be terminated immediately.
//!
//! ## Resolver status
//!
//! - Candidacy
//! - Active
//! - Terminated
//!
//! ## Events
//!
//! - ResolverCreated - A resolver is created.
//! - ResolverActivated - A resolver is activated.
//! - ResolverInactivated - A resolver is disabled.
//! - ResolverTerminated - A resolver is terminated.

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
		log,
		pallet_prelude::*,
		sp_runtime::traits::{Hash, Zero},
		sp_std::vec::Vec,
		traits::Randomness,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use pallet_identities::IdentitiesManager;
	use pallet_timestamp::{self as timestamp};
	use primitives::{Credibility, CurrencyId};
	use scale_info::TypeInfo;
	use sp_io::offchain_index;
	use sp_runtime::RuntimeDebug;

	#[cfg(feature = "std")]
	use serde::{Deserialize, Serialize};

	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId<Self::Hash>>;
		type IdentitiesManager: IdentitiesManager<Self::AccountId>;
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		#[pallet::constant]
		type PenaltyTokenLockTime: Get<MomentOf<Self>>;
		#[pallet::constant]
		type MinimumSelfStake: Get<BalanceOf<Self>>;
		#[pallet::constant]
		type ActivationStakeAmount: Get<BalanceOf<Self>>;
		#[pallet::constant]
		type UndelegateTime: Get<MomentOf<Self>>;
		/// The required credibility to become a resolver.
		#[pallet::constant]
		type RequiredCredibility: Get<Credibility>;
	}

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
	type MomentOf<T> = <T as pallet_timestamp::Config>::Moment;

	pub trait ResolversNetwork<AccountId, Hash> {
		fn get_resolver(
			payment_hash: Hash,
			selected: Vec<AccountId>,
		) -> Result<AccountId, DispatchError>;

		fn increase_credibility(resolver_id: &AccountId, amount: Credibility) -> DispatchResult;

		fn decrease_credibility(resolver_id: AccountId, amount: Credibility) -> DispatchResult;
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct PendingFund<T: Config> {
		pub owner: AccountOf<T>,
		pub amount: BalanceOf<T>,
		pub release_at: MomentOf<T>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Delegation<T: Config> {
		pub delegator: AccountOf<T>,
		pub amount: BalanceOf<T>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum ResolverStatus {
		Candidacy,
		Active,
		Terminated,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Resolver<T: Config> {
		pub application_digest: T::Hash,
		pub status: ResolverStatus,
		pub self_stake: BalanceOf<T>,
		// TODO: Considering change to HashMap for better performance.
		pub delegations: Vec<Delegation<T>>,
		pub total_stake: BalanceOf<T>,
		pub updated_at: MomentOf<T>,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn resolvers)]
	pub(super) type Resolvers<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, Resolver<T>>;

	#[pallet::storage]
	#[pallet::getter(fn active_resolvers)]
	pub(super) type ActiveResolvers<T: Config> = StorageValue<_, Vec<AccountOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_funds)]
	pub(super) type PendingFunds<T: Config> = StorageValue<_, Vec<PendingFund<T>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A resolver is created.
		ResolverCreated { account: AccountOf<T> },
		/// A resolver is activated.
		ResolverActivated { account: AccountOf<T> },
		/// A resolver is inactivated.
		ResolverInactivated { account: AccountOf<T> },
		/// A resolver is terminated.
		ResolverTerminated { account: AccountOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient balance to do the action.
		InsufficientBalance,
		/// The identity is required to become resolver.
		IdentityRequired,
		/// The self stake have to higher than required.
		CredibilityTooLow,
		/// The self stake have to higher than required.
		NotMeetMinimumSelfStake,
		/// There is no resolver related to the account.
		ResolverNotFound,
		/// Their is no delegation of the account to the resolver.
		DelegationNotFound,
		/// The requested amount exceed the delegated amount.
		InvalidAmount,
		/// The origin is not a resolver.
		NotAResolver,
		/// There is no active resolver at the moment.
		NoAnyActiveResolver,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			if let Err(err) = Self::run_offchain_worker() {
				log::error!(
					target: "Resolvers network offchain worker",
					"Fail to run offchain worker at block {:?}: {:?}",
					block_number,
					err,
				);
			} else {
				log::debug!(
					target: "Resolvers network offchain worker",
					"offchain worker start at block: {:?} already done!",
					block_number,
				);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn join_resolvers_network(
			origin: OriginFor<T>,
			application: Vec<u8>,
			self_stake: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::_create_resolver(sender, application, self_stake)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn delegate(
			origin: OriginFor<T>,
			resolver: AccountOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::_delegate(sender, resolver, amount)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn undelegate(
			origin: OriginFor<T>,
			resolver: AccountOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::_undelegate(sender, resolver, amount)?;
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn resign(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::_terminate_resolver(sender)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn run_offchain_worker() -> DispatchResult {
			Self::_release_pending_funds()?;
			Ok(())
		}

		fn _create_resolver(
			sender: AccountOf<T>,
			application: Vec<u8>,
			self_stake: BalanceOf<T>,
		) -> DispatchResult {
			// The identity is required to join resolver networks.
			ensure!(T::IdentitiesManager::has_identity(&sender), <Error<T>>::IdentityRequired);
			// The identity credibility must be higher than required level to join resolver
			// networks.
			let resolver_credibility = T::IdentitiesManager::get_credibility(&sender)?;
			ensure!(
				resolver_credibility > T::RequiredCredibility::get(),
				<Error<T>>::CredibilityTooLow
			);
			ensure!(self_stake >= T::MinimumSelfStake::get(), <Error<T>>::NotMeetMinimumSelfStake);
			ensure!(
				T::Currency::free_balance(CurrencyId::<T::Hash>::Native, &sender) >= self_stake,
				<Error<T>>::InsufficientBalance,
			);

			let now = <timestamp::Pallet<T>>::get();
			let application_digest = T::Hashing::hash_of(&application);

			offchain_index::set(&application_digest.encode(), &application);

			T::Currency::reserve(CurrencyId::<T::Hash>::Native, &sender, self_stake)?;

			let mut resolver = Resolver::<T> {
				application_digest,
				status: ResolverStatus::Candidacy,
				self_stake,
				total_stake: self_stake,
				delegations: [].to_vec(),
				updated_at: now,
			};

			if resolver.total_stake >= T::ActivationStakeAmount::get() {
				resolver.status = ResolverStatus::Active;
				Self::_add_active_resolver(sender.clone());
			};

			<Resolvers<T>>::insert(&sender, resolver);

			Self::deposit_event(Event::ResolverCreated { account: sender });

			Ok(())
		}

		fn _delegate(
			sender: AccountOf<T>,
			resolver_account: AccountOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(
				T::Currency::free_balance(CurrencyId::<T::Hash>::Native, &sender) >= amount,
				<Error<T>>::InsufficientBalance,
			);

			let mut resolver =
				Self::resolvers(&resolver_account).ok_or(<Error<T>>::ResolverNotFound)?;

			T::Currency::reserve(CurrencyId::<T::Hash>::Native, &sender, amount)?;

			let delegation_position = resolver
				.delegations
				.iter()
				.position(|delegation| delegation.delegator == sender);

			match delegation_position {
				Some(p) => {
					resolver.delegations[p].amount += amount;
				},
				None => {
					let delegation = Delegation::<T> { delegator: sender, amount };

					resolver.delegations.push(delegation);
				},
			}

			resolver.total_stake += amount;

			if resolver.total_stake >= T::ActivationStakeAmount::get() {
				resolver.status = ResolverStatus::Active;
				Self::_add_active_resolver(resolver_account.clone());
				Self::deposit_event(Event::ResolverActivated { account: resolver_account.clone() });
			};

			<Resolvers<T>>::insert(&resolver_account, resolver);

			Ok(())
		}

		fn _undelegate(
			sender: AccountOf<T>,
			resolver_account: AccountOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let mut resolver =
				Self::resolvers(&resolver_account).ok_or(<Error<T>>::ResolverNotFound)?;

			let delegation_position = resolver
				.delegations
				.iter()
				.position(|delegation| delegation.delegator == sender);

			match delegation_position {
				None => return Err(<Error<T>>::DelegationNotFound.into()),
				Some(p) => {
					if amount > resolver.delegations[p].amount {
						return Err(<Error<T>>::InvalidAmount.into())
					}

					resolver.delegations[p].amount -= amount;
					resolver.total_stake -= amount;

					let release_at = <timestamp::Pallet<T>>::get() + T::UndelegateTime::get();

					let pending_fund = PendingFund::<T> { owner: sender, amount, release_at };

					<PendingFunds<T>>::mutate(|pending_funds| {
						pending_funds.push(pending_fund);
					});

					if resolver.total_stake < T::ActivationStakeAmount::get() {
						resolver.status = ResolverStatus::Candidacy;
						Self::_remove_active_resolver(resolver_account.clone());
						Self::deposit_event(Event::ResolverInactivated {
							account: resolver_account.clone(),
						});
					};

					resolver.delegations.retain(|delegation| delegation.amount > Zero::zero());

					<Resolvers<T>>::insert(&resolver_account, resolver);
				},
			}

			Ok(())
		}

		fn _terminate_resolver(resolver_account: AccountOf<T>) -> DispatchResult {
			let mut resolver =
				Self::resolvers(&resolver_account).ok_or(<Error<T>>::NotAResolver)?;

			let release_at = <timestamp::Pallet<T>>::get() + T::UndelegateTime::get();
			let resolver_pending_fund = PendingFund::<T> {
				owner: resolver_account.clone(),
				amount: resolver.self_stake,
				release_at,
			};
			<PendingFunds<T>>::mutate(|pending_funds| {
				pending_funds.push(resolver_pending_fund);
			});

			for delegation in resolver.delegations.iter() {
				let pending_fund = PendingFund::<T> {
					owner: delegation.delegator.clone(),
					amount: delegation.amount,
					release_at,
				};

				<PendingFunds<T>>::mutate(|pending_funds| {
					pending_funds.push(pending_fund);
				});
			}

			resolver.total_stake = Zero::zero();
			resolver.self_stake = Zero::zero();
			resolver.delegations = [].to_vec();
			resolver.status = ResolverStatus::Terminated;

			Self::_remove_active_resolver(resolver_account.clone());
			<Resolvers<T>>::insert(&resolver_account, resolver);
			Self::deposit_event(Event::ResolverTerminated { account: resolver_account });

			Ok(())
		}

		fn _add_active_resolver(resolver: AccountOf<T>) {
			<ActiveResolvers<T>>::mutate(|resolvers| resolvers.push(resolver));
		}

		fn _remove_active_resolver(resolver: AccountOf<T>) {
			<ActiveResolvers<T>>::mutate(|resolvers| resolvers.retain(|r| *r != resolver));
		}

		fn _release_pending_funds() -> DispatchResult {
			let mut pending_funds = <PendingFunds<T>>::get();
			let now = <timestamp::Pallet<T>>::get();

			pending_funds.retain(|fund| {
				let can_release = now >= fund.release_at;
				if can_release {
					T::Currency::unreserve(CurrencyId::<T::Hash>::Native, &fund.owner, fund.amount);
				}

				!can_release
			});

			<PendingFunds<T>>::set(pending_funds);

			Ok(())
		}
	}

	impl<T: Config> ResolversNetwork<T::AccountId, T::Hash> for Pallet<T> {
		fn get_resolver(
			payment_hash: T::Hash,
			selected: Vec<T::AccountId>,
		) -> Result<T::AccountId, DispatchError> {
			let mut active_resolvers = <ActiveResolvers<T>>::get();
			active_resolvers.retain(|id| !selected.contains(id));
			ensure!(!active_resolvers.is_empty(), <Error<T>>::NoAnyActiveResolver);
			let (output, _block_number) = T::Randomness::random(payment_hash.as_ref());
			let random_number: usize = output.as_ref().iter().map(|x| *x as usize).sum();

			Ok(active_resolvers[random_number % active_resolvers.len()].clone())
		}

		fn increase_credibility(
			resolver_account_id: &T::AccountId,
			amount: Credibility,
		) -> DispatchResult {
			T::IdentitiesManager::increase_credibility(resolver_account_id, amount)?;
			Ok(())
		}

		fn decrease_credibility(
			resolver_account_id: T::AccountId,
			amount: Credibility,
		) -> DispatchResult {
			T::IdentitiesManager::decrease_credibility(&resolver_account_id, amount)?;
			let credibility = T::IdentitiesManager::get_credibility(&resolver_account_id)?;
			if credibility < T::RequiredCredibility::get() {
				Self::_terminate_resolver(resolver_account_id)?;
			}
			Ok(())
		}
	}
}
