use crate::{mock::*};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_default_value() {
	ExtBuilder.build().execute_with(|| {
	});
}

#[test]
fn correct_error_for_none_value() {
	ExtBuilder.build().execute_with(|| {

	});
}
