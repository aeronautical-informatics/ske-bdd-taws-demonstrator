use opentaws::prelude::*;
use xng_rs::prelude::*;

use p_taws::*;

mod tester;

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn PartitionMain() -> isize {
    tester::test();

    xng_rs::partition::halt(partition::my_id().unwrap()).unwrap();
    0
}
