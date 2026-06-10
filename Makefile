CARGO ?= cargo
RUSTDOCFLAGS ?= -D warnings

.DEFAULT_GOAL := help

.PHONY: help build check fmt fmt-check lint doc test ci install install-hooks

help: ## Show this help table
	@awk 'BEGIN {FS = ":.*##"; printf "Available targets:\n\n"} \
	      /^[a-z][a-z0-9-]*:.*##/ { printf "  make %-15s — %s\n", $$1, $$2 }' \
	      $(MAKEFILE_LIST)

build: ## Compile (debug profile)
	$(CARGO) build

check: ## Type-check including tests
	$(CARGO) check --all-targets

fmt: ## Format with rustfmt
	$(CARGO) fmt

fmt-check: ## Verify formatting without rewriting
	$(CARGO) fmt -- --check

lint: ## Clippy — warnings denied
	$(CARGO) clippy --all-targets -- -D warnings

doc: ## Build rustdoc — warnings denied
	RUSTDOCFLAGS="$(RUSTDOCFLAGS)" $(CARGO) doc --no-deps

test: ## Run tests
	$(CARGO) test

ci: fmt-check lint test doc ## Same gate CI runs (fmt + lint + test + doc)
	@echo "ci: all checks passed"

install: ## Install tonin-helm to ~/.cargo/bin
	$(CARGO) install --path .

install-hooks: ## Wire up git hooks (run once after cloning)
	cp scripts/pre-commit .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	cp scripts/commit-msg .git/hooks/commit-msg
	chmod +x .git/hooks/commit-msg
	@echo "hooks installed: pre-commit, commit-msg"
