## LRP protocol

### Overview

LRP protocol allows buyer and seller to make a p2p payment while the cryptocurrencies of the buyer have locked until the seller delivers the order. The below diagram will explain the logic of LRP protocol when the sellers integrate with Libra Network.

![Project Libra-LRP Protocol](https://user-images.githubusercontent.com/92568442/148349639-145690aa-98c3-4e13-b9a3-ccfa01d55f6a.png)

### Payment state transition

![state-transition](https://user-images.githubusercontent.com/92568442/148345661-fd24292a-389b-44ef-95a5-5d8422f546c6.png)

## Data structure of payment

```rs
Payment<T: Config> {
  pub id: u128,
  pub payer: AccountOf<T>,
  pub payee: AccountOf<T>,
  pub amount: BalanceOf<T>,
  pub currency_id: CurrencyIdOf<T>,
  pub description: Vec<u8>,
  pub status: PaymentStatus,
  pub receipt_hash: <Runtime as system::Config>::Hash,
  pub created_at: MomentOf<T>,
  pub updated_at: MomentOf<T>,
  pub updated_by: AccountOf<T>,
}
```

### Usage

**create_payment**
```rs
pub fn create_payment(
  origin: <Runtime as system::Config>::Origin,
  payee: <Runtime as system::Config>::AccountId,
  amount: <Runtime as system::Config>::Balance,
  currency_id: <Runtime as system::Config>::Hash,
  description: Vec<u8>,
  receipt: Vec<u8>,
) -> DispatchResult
```


**accept_payment**
```rs
pub fn create_payment(
  origin: OriginFor<T>,
  payment_hash: <Runtime as system::Config>::Hash,
) -> DispatchResult
```

**reject_payment**
```rs
pub fn reject_payment(
  origin: <Runtime as system::Config>::Origin,
  payment_hash: <Runtime as system::Config>::Hash,
) -> DispatchResult
```

**reject_payment**
```rs
pub fn reject_payment(
  origin: <Runtime as system::Config>::Origin,
  payment_hash: <Runtime as system::Config>::Hash,
) -> DispatchResult
```

**full_fill_payment**
```rs
pub fn complete_payment(
  origin: <Runtime as system::Config>::Origin,
  payment_hash: <Runtime as system::Config>::Hash,
) -> DispatchResult
```

**complete_payment**
```rs
pub fn complete_payment(
  origin: <Runtime as system::Config>::Origin,
  payment_hash: <Runtime as system::Config>::Hash,
) -> DispatchResult
```