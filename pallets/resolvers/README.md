# Resolvers Network

## Overview
Resolvers network is a decentralized arbitrators network that allows an arbitrator to stake some native tokens to join and resolve payment conflicts between parties to receive the fee from dispute parties. A resolver can just stake an amount that meets the minimum requirements and run a community crowd loan to get enough delegations to become an active resolver. The delegators will share the rewards with the resolver.

## Usage

### Resovolser

**Join resolver networks**
```rs
pub fn join_resolvers_network(
  origin: OriginFor<T>,
  application: Vec<u8>,
  self_stake: Balance,
)
```
**Quit resolver network**
```rs
pub fn resign(origin: OriginFor<T>)
```

### Delegator
**Delegate to a resolver**
```rs
pub fn delegate(
  origin: OriginFor<T>,
  resolver: AccountId,
  amount: Balance,
)
```
**Undelegate tokens from a resolver**
`WARNING: Tokens will be released after UndelegateTime.`
```rs
pub fn undelegate(
  origin: OriginFor<T>,
  resolver: AccountId,
  amount: Balance,
)
```

## Traits
```rs
pub trait ResolversNetwork<AccountId, Hash> {
  fn get_resolver(
    payment_hash: Hash,
    selected: Vec<AccountId>,
  ) -> Result<AccountId, DispatchError>;

  fn increase_credibility(resolver_id: AccountId, amount: Credibility) -> DispatchResult;

  fn reduce_credibility(resolver_id: AccountId, amount: Credibility) -> DispatchResult;
}
```
