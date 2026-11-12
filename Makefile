# Every task runs inside the containers defined in .devcontainer/compose.yaml:
# the dev service for checks and builds, the diagrams service for PlantUML.
# The host needs only Docker, Compose, Make and Git.

COMPOSE := USER_UID=$(shell id -u) USER_GID=$(shell id -g) \
	docker compose -f .devcontainer/compose.yaml
EXEC := $(COMPOSE) exec -T dev
DIAGRAMS := $(filter-out docs/diagrams/style.puml,$(wildcard docs/diagrams/*.puml))
MARKDOWN := "**/*.md" "\#.tmp/**" "\#target/**" "\#dist/**"


.PHONY: dev image shell runtime fmt fmt-check lint type test coverage docs-lint ascii-check traceability test-int test-e2e test-all \
	build dist bless coverage-branches miri fuzz runtime-size diagrams diagrams-check unicode-tables unicode-tables-check format-cases format-cases-check stdlib-names stdlib-names-check errno-texts errno-texts-check mutants bench check hooks clean

dev: ## Start the dev service, building its image the first time
	@$(COMPOSE) up -d --quiet-pull dev 2>/dev/null

image: ## Rebuild the dev image after a change to .devcontainer
	$(COMPOSE) build dev

shell: dev ## Open a shell in the dev service
	$(COMPOSE) exec dev bash

runtime: dev ## Build the runtime archive for each target and stage it where <target>/*/typy finds it
	@$(EXEC) scripts/build-runtime.sh

# The conformance, determinism and documentation tests run <target>/debug/typy as a
# program, so every test target builds it first.
build: runtime ## Build the typy compiler, <target>/debug/typy, with its runtime archive staged
	@$(EXEC) cargo build -p typy-cli --locked

fmt: dev ## Format the Rust code
	@$(EXEC) cargo fmt --all

fmt-check: dev ## Fail when the Rust code is not formatted
	@$(EXEC) cargo fmt --all --check

lint: dev ## Run Clippy on every crate and test, with warnings as errors
	@$(EXEC) cargo clippy --workspace --all-targets --locked -- -D warnings

type: dev ## Type check every crate and test
	@$(EXEC) cargo check --workspace --all-targets --locked

test: build ## Run the unit tests
	@$(EXEC) cargo nextest run --workspace --locked

test-int: build ## Run the integration tests: the conformance suite, its sanitizer run, golden files and the pipeline
	@$(EXEC) cargo nextest run -p typy-conformance -p typy-driver --locked

test-e2e: runtime ## Run the end-to-end tests of the typy executable
	@$(EXEC) cargo nextest run -p typy-cli --test cli --locked

test-all: test miri ## Run every test level that runs on each commit, and Miri
coverage: build ## Run the tests with coverage; fail below 95 percent of lines (NFR-11)
	@$(EXEC) cargo llvm-cov nextest --workspace --locked --status-level fail \
		--final-status-level fail --fail-under-lines 95 --summary-only \
		--ignore-filename-regex 'typy-conformance/'

coverage-branches: build ## Measure line and branch coverage with nightly; fail below 95 percent of either (NFR-11)
	@$(EXEC) scripts/coverage-branches.sh

docs-lint: dev ## Lint every Markdown file with markdownlint-cli2
	@$(EXEC) markdownlint-cli2 $(MARKDOWN)

ascii-check: dev ## Fail on non-ASCII text in sources, documents and scripts
	@$(EXEC) scripts/check-ascii.sh

traceability: dev ## Fail on requirements without tests and IDs that are not defined
	@$(EXEC) scripts/check-traceability.sh

diagrams: ## Render every PlantUML source to the SVG next to it
	@$(COMPOSE) run --rm --quiet-pull diagrams -tsvg -nometadata -failfast2 $(DIAGRAMS)

diagrams-check: diagrams dev ## Fail when an SVG in the index differs from its source
	@$(EXEC) scripts/check-diagrams.sh

dist: dev ## Build the release archive of the compiler in dist/, and run a program with it unpacked
	@$(EXEC) scripts/package-release.sh

bless: dev ## Rewrite the golden files from the current compiler; review the diff before committing
	@$(EXEC) env TYPY_BLESS=1 cargo test -p typy-driver --test golden --locked

miri: dev ## Run the runtime's unit tests under Miri with the pinned nightly toolchain (ADR-0006)
	@$(EXEC) scripts/miri.sh

# `docker compose exec` passes no variable of the host's environment, so the
# fuzzing time is handed to the container by name.
FUZZ_SECONDS ?= 60

fuzz: dev ## Fuzz the source, checking and lowering stages for FUZZ_SECONDS seconds, 60 by default (NFR-09)
	@$(COMPOSE) exec -T -e FUZZ_SECONDS=$(FUZZ_SECONDS) dev scripts/fuzz.sh

MUTANTS_PACKAGE ?= typy-diagnostics

mutants: ## Run cargo-mutants on MUTANTS_PACKAGE in a container limited to 6 GB, without network; the report goes to .tmp/mutants/
	@scripts/mutants.sh $(MUTANTS_PACKAGE)

bench: runtime ## Time the release-built compiler on generated 10,000-line projects against the budgets of NFR-04, NFR-05 and NFR-14
	@$(EXEC) cargo build --release -p typy-cli --locked
	@$(EXEC) python3 tests/bench/build_time.py

runtime-size: build ## Fail when the runtime a hello world links is over the size budgets of NFR-02
	@$(EXEC) python3 scripts/check-runtime-size.py

unicode-tables: dev ## Write the runtime's Unicode tables from CPython 3.15's unicodedata (OI-15)
	@$(EXEC) python3.15 scripts/generate-unicode-tables.py

unicode-tables-check: dev ## Fail when the committed Unicode tables differ from CPython 3.15's
	@$(EXEC) python3.15 scripts/generate-unicode-tables.py --check

format-cases: dev ## Write the runtime's table of format() results from CPython 3.12 to 3.15 (LEX-07)
	@$(EXEC) python3.15 scripts/generate-format-cases.py

format-cases-check: dev ## Fail when the committed table of format() results differs from CPython's
	@$(EXEC) python3.15 scripts/generate-format-cases.py --check

stdlib-names: dev ## Write the loader's table of CPython 3.12 to 3.15's standard library module names (LIB-08)
	@$(EXEC) python3.15 scripts/generate-stdlib-names.py

stdlib-names-check: dev ## Fail when the committed table of standard library module names differs from CPython's
	@$(EXEC) python3.15 scripts/generate-stdlib-names.py --check

errno-texts: dev ## Write the runtime's table of glibc's error texts, which OSError shows (ADR-0019)
	@$(EXEC) python3.15 scripts/generate-errno-texts.py

errno-texts-check: dev ## Fail when the committed table of error texts differs from glibc's
	@$(EXEC) python3.15 scripts/generate-errno-texts.py --check

check: fmt-check lint coverage-branches docs-lint ascii-check traceability diagrams-check unicode-tables-check format-cases-check stdlib-names-check errno-texts-check runtime-size ## Run the whole quality gate

hooks: ## Use the hooks in .githooks for this clone
	git config core.hooksPath .githooks

clean: ## Stop the containers and remove scratch previews and release archives
	$(COMPOSE) down
	rm -rf .tmp/preview dist
