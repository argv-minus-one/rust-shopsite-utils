use assert_cmd::Command;
use std::path::PathBuf;

fn test_aa_location() -> PathBuf {
	[env!("CARGO_MANIFEST_DIR"), "..", "shopsite-aa", "tests", "test.aa"].iter().collect()
}

fn get_cmd() -> Command {
	Command::cargo_bin("shopsite-aa2json").unwrap()
}

fn run_test(cmd: &mut Command, expected_output: &str) {
	let results = cmd.unwrap();

	assert!(results.status.success());
	assert_eq!(String::from_utf8(results.stdout).unwrap(), expected_output);
	assert_eq!(&results.stderr[..], &[], "standard error output should have been empty");
}

#[test]
fn run_compact() {
	run_test(
		get_cmd().arg(test_aa_location()),
		include_str!("expected-compact.json")
	)
}

#[test]
fn run_pretty_spaces() {
	run_test(
		get_cmd().args(&["-p", "-s", "3"]).arg(test_aa_location()),
		include_str!("expected-pretty-spaces.json")
	)
}

#[test]
fn run_pretty_tabs() {
	run_test(
		get_cmd().arg("-tp").arg(test_aa_location()),
		include_str!("expected-pretty-tabs.json")
	)
}
