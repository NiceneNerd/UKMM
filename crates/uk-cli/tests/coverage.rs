//! The architecture test for the CLI frontend: every [`Command`] variant
//! must be exercised by a CLI subcommand. `Command::variant_names()` is an
//! exhaustive match, so adding a variant breaks the uk-app build until it is
//! named; this test then fails until `COMMAND_COVERAGE` (and a subcommand)
//! covers it.

use uk_app::Command;
use uk_cli::COMMAND_COVERAGE;

#[test]
fn every_command_variant_has_a_subcommand() {
    let variants = Command::variant_names();
    let covered: std::collections::BTreeSet<_> = COMMAND_COVERAGE.iter().map(|(v, _)| *v).collect();
    let mut missing: Vec<_> = variants.iter().filter(|v| !covered.contains(**v)).collect();
    missing.sort_unstable();
    assert!(
        missing.is_empty(),
        "Command variants without a CLI subcommand: {missing:?}. Add a subcommand to uk-cli and \
         an entry to COMMAND_COVERAGE."
    );
    // And no stale entries: every table row names a real variant.
    let stale: Vec<_> = covered.iter().filter(|v| !variants.contains(v)).collect();
    assert!(
        stale.is_empty(),
        "COMMAND_COVERAGE names unknown variants: {stale:?}"
    );
}
