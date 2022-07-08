# Identities
## Overview
The identities pallet is a module that allows an individual/organization can create and manage their own on-chain self-sovereign identity. There are 2 key factors of the modules:

- Identity Data: The data provided by the identity owner. The data can be anything from email, and domain to legal data on the entity... The identity data in be used to do risk evaluation before making a transaction with the identity owner.

- Identity Verification Service: 3rd services who deposit some tokens and take responsibility to verify specified fields of identity data to earn rewards. It can be an automation service such as email and domain verification or a KYC service.
## Usage
### Identity Owner

**Create a new identity**
```rs
pub fn create_identity(
  origin: OriginFor<T>,
  name: Vec<u8>,
  identity_type: IdentityType,
  data: Vec<IdentityFieldInput>,
)
```

**Update existed identity**

`Warning: This will replace the old identy with the new one.`
```rs
pub fn update_identity(
  origin: OriginFor<T>,
  name: Option<Vec<u8>>,
  data: Option<Vec<IdentityFieldInput>>,
)
```
**Update a data field of an existed identity**
```rs
pub fn update_identity_data(
  origin: OriginFor<T>,
  position: u64,
  data_field: IdentityFieldInput,
)
```

**Add a new data field to an existed identity**
```rs
pub fn add_identity_data(
  origin: OriginFor<T>,
  data_field: IdentityFieldInput,
)
```

**Remove an existed identity**
```rs
pub fn remove_identy(origin: OriginFor<T>)
```

**Request an evaluator to verify identity data**
```rs
pub fn request_to_verify(
  origin: OriginFor<T>,
  positions: Vec<u64>,
  evaluator: AccountId,
)
```

### Identity Verify Services

**Bond native tokens to become evaluator**
```rs
pub fn create_evaluator(
  origin: OriginFor<T>,
  name: Vec<u8>,
  about: Vec<u8>,
  rate: Balance,
)
```

**Verify data of an identity**
```rs
pub fn verify_data(
  origin: OriginFor<T>,
  account: AccountId,
  transcript: Vec<(u64, bool)>
)
```