# Every task runs inside the containers defined in .devcontainer/compose.yaml:
# the dev service for checks and builds, the diagrams service for PlantUML.
# The host needs only Docker, Compose, Make and Git.

COMPOSE := USER_UID=$(shell id -u) USER_GID=$(shell id -g) \
	docker compose -f .devcontainer/compose.yaml
EXEC := $(COMPOSE) exec -T dev
DIAGRAMS := $(filter-out docs/diagrams/style.puml,$(wildcard docs/diagrams/*.puml))
MARKDOWN := "**/*.md" "\#.tmp/**" "\#target/**"


.PHONY: dev image shell runtime fmt fmt-check lint type test coverage docs-lint ascii-check traceability \
	build diagrams diagrams-check unicode-tables unicode-tables-check check hooks clean

dev: ## Start the dev service, building its image the first time
	@$(COMPOSE) up -d --quiet-pull dev 2>/dev/null

image: ## Rebuild the dev image after a change to .devcontainer
	$(COMPOSE) build dev

shell: dev ## Open a shell in the dev service
	$(COMPOSE) exec dev bash

# The archive is staged where a compiler in <target>/debug or <target>/release finds it,
# <target>/lib/typy/<triple>/, as an installed compiler finds it in <prefix>/lib/typy/<triple>/;
# the copy under llvm-cov-target serves the compiler that `make coverage` builds there.
runtime: dev ## Build the runtime archive and stage it where <target>/*/typy finds it
	@$(EXEC) cargo rustc -p typy-runtime --profile runtime --locked --crate-type staticlib
	@$(EXEC) bash -c 'target="$${CARGO_TARGET_DIR:-target}"; install -D -m 644 \
		"$$target/runtime/libtypy_runtime.a" \
		"$$target/lib/typy/x86_64-unknown-linux-gnu/libtypy_runtime.a"; \
		install -D -m 644 "$$target/runtime/libtypy_runtime.a" \
		"$$target/llvm-cov-target/lib/typy/x86_64-unknown-linux-gnu/libtypy_runtime.a"'

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

test: runtime ## Run the unit tests
	@$(EXEC) cargo nextest run --workspace --locked

coverage: runtime ## Run the tests with coverage; fail below 95 percent of lines (NFR-11)
	@$(EXEC) cargo llvm-cov nextest --workspace --locked --status-level fail \
		--final-status-level fail --fail-under-lines 95 --summary-only

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

unicode-tables: dev ## Write the runtime's Unicode tables from CPython 3.15's unicodedata (OI-15)
	@$(EXEC) python3.15 scripts/generate-unicode-tables.py

unicode-tables-check: dev ## Fail when the committed Unicode tables differ from CPython 3.15's
	@$(EXEC) python3.15 scripts/generate-unicode-tables.py --check

check: fmt-check lint coverage docs-lint ascii-check traceability diagrams-check unicode-tables-check ## Run the whole quality gate

hooks: ## Use the hooks in .githooks for this clone
	git config core.hooksPath .githooks

clean: ## Stop the containers and remove scratch previews
	$(COMPOSE) down
	rm -rf .tmp/preview
