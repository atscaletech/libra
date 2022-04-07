# Libra
Libra Payment Network is a decentralized-first payment system. Libra allows people to accept cryptocurrencies as a payment method and handle real-world eCommerce transactions without hassle.
## Overview
Blockchain is revolutionizing eCommerce, making payment safer and faster while bringing greater access to global consumers.  
Due to the nature of digital currency protocols, transactions cannot be canceled or altered once they are initiated.  
However, global eCommerce data shows that at least 30% of all products ordered online are returned.  

How can we adopt blockchain to eCommerce with such a barrier?

Libra was born to tackle this problem and help facilitate blockchain adoption in the eCommerce industry.  
Libra is a decentralized payment network. Through its SDK, Libra allows sellers to accept cryptocurrency payments in minutes.  
Libra includes a Lock and Release Payment (LRP) Protocol and Resolvers Network at its core.  
LRP Protocol helps the buyer to purchase with confidence. It also helps the seller to increase conversion and do proper order handling.  
Resolvers Network leverages the power of blockchain and the community to resolve transaction conflict in a quick and efficient method without involving any financial institution.  

Libra bridges the gap between blockchain and eCommerce to enable all people to exchange value and transact globally, securely, at significantly lower cost, and more inclusively than traditional financial systems allow.

The project's scope is to build three core components that define the foundation of Libra Network: LRP protocol, Resolver Networks, and Javascript SDK. From these components, people can easily integrate the cryptocurrencies payment to their business while their customers are protected by Libra Network.

- [LRP Protocol](https://github.com/atscaletech/libra/blob/main/pallets/lrp/README.md)
- Resolver Network (WIP)
- [Javascript SDK](https://github.com/atscaletech/libra-js)
## Installation

#### Clone Repo

```
git clone --recursive git@github.com:atscaletech/libra.git
cd libra
```

#### Setup environment

```
sudo apt update && sudo apt install -y git clang curl libssl-dev llvm libudev-dev
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
rustup default stable
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

#### Build

```
cargo build --release
```

#### Run dev chain

```
./target/release/libra --dev --tmp
```

#### Run tests

```
cargo test --release
```
