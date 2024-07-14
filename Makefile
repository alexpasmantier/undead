DEBUGGING_PATH = /some/path/to/a/dir

debug:
	@cargo build
	@make run-debug-binary

run:
	@cargo build --release
	@make run-release-binary

run-release-binary:
	@./target/release/undead ${DEBUGGING_PATH}

run-debug-binary:
	@./target/debug/undead ${DEBUGGING_PATH}
