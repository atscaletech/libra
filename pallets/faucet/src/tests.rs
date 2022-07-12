use super::*;

use frame_support::{assert_ok};
use mock::{Currencies, ExtBuilder, Faucet, Origin, System, ALICE};
use primitives::CurrencyId;
use sp_core::H256;
use orml_traits::{MultiCurrency};

#[test]
fn faucet_works() {
	ExtBuilder.build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(Faucet::faucet(
			Origin::signed(ALICE),
			1000,
			CurrencyId::Registered(H256::zero())
		));
		assert_eq!(Currencies::free_balance(CurrencyId::Native, &ALICE), 1000);
		assert_eq!(Currencies::free_balance(CurrencyId::Registered(H256::zero()), &ALICE), 1000);
	});
}
