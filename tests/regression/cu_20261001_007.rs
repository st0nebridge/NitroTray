//! @module cu_20261001_007
//! @description Regression test (mutation runs leaked process trees): every mutant's UI test run must end itself.
//! On Windows, Stryker's kill of a timed-out command run can fail under memory pressure and abandon the
//! whole `cmd` -> `node --test` tree; in another project those piled up to 229 processes and 25 GB. The
//! command cancels a hung test and exits past open handles, with Stryker's timeout above the tests' own.

const CONFIG: &str = include_str!("../../stryker.config.json");

/// The number after `"key":` in the config, which is flat enough to read without a JSON parser.
fn number(key: &str) -> u64 {
    let at = CONFIG.find(&format!("\"{key}\"")).unwrap_or_else(|| panic!("{key} missing"));
    let rest = &CONFIG[at..];
    let start = rest.find(':').expect("colon") + 1;
    rest[start..].trim_start().chars().take_while(char::is_ascii_digit).collect::<String>().parse().expect(key)
}

#[test]
fn mutant_test_runs_end_themselves() {
    assert!(CONFIG.contains("\"testRunner\": \"command\""), "this test is about the command runner");
    let command = CONFIG.lines().find(|line| line.contains("\"command\":")).expect("command line");
    assert!(command.contains("node --test --test-timeout=10000 --test-force-exit "), "{command}");
}

#[test]
fn stryker_timeout_sits_above_the_tests_own() {
    assert!(number("timeoutMS") > 10000, "timeoutMS {}", number("timeoutMS"));
    assert!(number("concurrency") <= 4, "concurrency {}", number("concurrency"));
}
