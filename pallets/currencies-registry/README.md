## Currencies registry 
### Overview
The currencies registry allows the registrars to create their own currencies by bonding some native tokens. When the registrars remove currencies, they will get back the bonded tokens. The merchants need to accept the currencies before people create payments with these currencies in the LRP protocol.

### Usage

**create_currency**
```rs
pub fn create_currency(
  origin: <Runtime as system::Config>::Origin,
  name: Vec<u8>,
  symbol: Vec<u8>,
  decimal: Vec<u8>,
) -> DispatchResult
```

**remove_currency**
```rs
pub fn remove_currency(
  origin: <Runtime as system::Config>::Origin,
  currency_id: <Runtime as system::Config>::Hash
) -> DispatchResult
```

**accept_currency**
```rs
pub fn accept_currency(
  origin: <Runtime as system::Config>::Origin,
  currency_id: <Runtime as system::Config>::Hash
) -> DispatchResult
```