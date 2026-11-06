# Every task runs inside the containers defined in .devcontainer/compose.yaml:
# the dev service for checks and builds, the diagrams service for PlantUML.
# The host needs only Docker, Compose, Make and Git.

COMPOSE := USER_UID=$(shell id -u) USER_GID=$(shell id -g) \
	docker compose -f .devcontainer/compose.yaml
EXEC := $(COMPOSE) exec -T dev
DIAGRAMS := $(filter-out docs/diagrams/style.puml,$(wildcard docs/diagrams/*.puml))
MARKDOWN := "**/*.md" "\#.tmp/**" "\#target/**"

.PHONY: dev image shell docs-lint ascii-check traceability diagrams diagrams-check check hooks clean

dev: ## Start the dev service, building its image the first time
	@$(COMPOSE) up -d --quiet-pull dev 2>/dev/null

image: ## Rebuild the dev image after a change to .devcontainer
	$(COMPOSE) build dev

shell: dev ## Open a shell in the dev service
	$(COMPOSE) exec dev bash

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

check: docs-lint ascii-check traceability diagrams-check ## Run the whole quality gate

hooks: ## Use the hooks in .githooks for this clone
	git config core.hooksPath .githooks

clean: ## Stop the containers and remove scratch previews
	$(COMPOSE) down
	rm -rf .tmp/preview
