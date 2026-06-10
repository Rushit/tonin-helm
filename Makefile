CARGO ?= cargo
RUSTDOCFLAGS ?= -D warnings
VERSION ?=

.DEFAULT_GOAL := help

.PHONY: help build check fmt fmt-check lint doc test ci install install-hooks \
        show-version version release publish

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

# ---------------------------------------------------------------------------
# Versioning + release
# ---------------------------------------------------------------------------

show-version: ## Print the current version (from /VERSION)
	@cat VERSION

version: ## Bump VERSION + Cargo.toml and commit (VERSION=X.Y.Z)
	@test -n "$(VERSION)" || \
	  (echo "usage: make version VERSION=X.Y.Z" >&2; exit 1)
	./scripts/bump-version.sh "$(VERSION)"

release: version ## Bump, tag, and push vX.Y.Z to origin (fires release CI)
	@echo "→ tagging v$(VERSION)"
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "→ pushing main"
	git push origin main
	@echo "→ pushing tag v$(VERSION)"
	git push origin "v$(VERSION)"
	@echo
	@echo "✓ v$(VERSION) released. .github/workflows/release.yml is running:"
	@echo "  https://github.com/Rushit/tonin-helm/actions"

publish: ## Publish tonin-helm to crates.io (requires CARGO_REGISTRY_TOKEN)
	@test -n "$${CARGO_REGISTRY_TOKEN}" || \
	  (echo "CARGO_REGISTRY_TOKEN not set" >&2; exit 1)
	$(CARGO) publish
