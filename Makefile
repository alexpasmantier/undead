DEBUGGING_FILE = /home/alex/code/python/data-doctrine

debug:
	@cargo build
	@make run-debug-binary

run:
	@cargo build --release
	@make run-release-binary

run-release-binary:
	@./target/release/undead ${DEBUGGING_FILE}

run-debug-binary:
	@./target/debug/undead ${DEBUGGING_FILE}
