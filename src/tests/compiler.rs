use clarity::{
    types::StacksEpochId,
    vm::{
        costs::LimitedCostTracker,
        types::{QualifiedContractIdentifier, StandardPrincipalData},
        ClarityVersion, ContractName,
    },
};

use crate::compiler::{analyze_contract, compile};

use super::datastore::Datastore;

#[test]
fn test_compile() {
    let contract_src = "
    (define-public (hello-world (arg1 int) (arg2 (string-ascii 10)))
  (begin (print (+ 2 arg1))
         (ok arg1)))
    ";

    let contract_id = QualifiedContractIdentifier::new(
        StandardPrincipalData::transient(),
        ContractName::from("add"),
    );
    let mut datastore = Datastore::new();
    let cost_tracker = LimitedCostTracker::new_free();

    let analyze_result = analyze_contract(
        contract_src,
        &contract_id,
        cost_tracker,
        ClarityVersion::Clarity2,
        StacksEpochId::Epoch24,
        &mut datastore,
    )
    .unwrap();

    let compile_result = compile(&analyze_result.contract_analysis).unwrap();
}
