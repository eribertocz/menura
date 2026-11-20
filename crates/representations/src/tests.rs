//! Unit tests of the crate as a whole (S9).

use std::fmt::Debug;

use crate::bound_query::BoundQuery;
use crate::logical_plan::PlanNode;
use crate::paths::Branches;
use crate::query_tree::QueryTree;

fn assert_plain_data<Representation: Clone + Debug + Eq>() {}

#[test]
fn s9_every_compile_stage_output_is_a_data_type_of_this_crate() {
    assert_plain_data::<QueryTree>();
    assert_plain_data::<BoundQuery>();
    assert_plain_data::<Branches>();
    assert_plain_data::<PlanNode>();
}
