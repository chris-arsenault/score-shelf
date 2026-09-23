RUST_CLIPPY_FLAGS := -D warnings -W clippy::cognitive_complexity -W clippy::too_many_lines
RUST_MAX_FILE_LINES := 400

.PHONY: ci lint lint-fix rust-lines-check telemetry-adoption-check fmt format format-check typecheck test backend-test frontend-test db-test scripts-check docs-check terraform-fmt-check build deploy

ci: lint telemetry-adoption-check fmt typecheck test scripts-check docs-check terraform-fmt-check

lint: rust-lines-check
	cd backend && cargo clippy --workspace --all-targets -- $(RUST_CLIPPY_FLAGS)
	cd frontend && pnpm exec eslint .

lint-fix:
	cd frontend && pnpm exec eslint . --fix

telemetry-adoption-check:
	cargo install --git https://github.com/chris-arsenault/ahara-infra.git ahara-lambda-telemetry --bin ahara-telemetry-adoption-check --locked --force
	ahara-telemetry-adoption-check backend

rust-lines-check:
	@violations=$$(find backend -path "*/target*" -prune -o -name "*.rs" -type f -print | while IFS= read -r file; do lines=$$(wc -l < "$$file"); if [ "$$lines" -gt "$(RUST_MAX_FILE_LINES)" ]; then printf "%s %s\n" "$$lines" "$$file"; fi; done); \
	if [ -n "$$violations" ]; then printf "Rust files exceed %s lines:\n%s\n" "$(RUST_MAX_FILE_LINES)" "$$violations"; exit 1; fi

fmt: format-check

format:
	cd backend && cargo fmt
	cd frontend && pnpm exec prettier --write .

format-check:
	cd backend && cargo fmt -- --check
	cd frontend && pnpm exec prettier --check .

typecheck:
	cd frontend && pnpm exec tsc --noEmit

test: backend-test frontend-test

backend-test:
	cd backend && cargo test --workspace --lib --bins

frontend-test:
	cd frontend && pnpm exec vitest run

db-test:
	cd backend && cargo test -p api --test store_pg

scripts-check:
	bash -n scripts/deploy.sh scripts/shelf.sh

docs-check:
	test -f README.md
	test -f AGENTS.md
	test -f CLAUDE.md
	test -f docs/README.md
	test -f docs/architecture.md
	test -f docs/adr/README.md

terraform-fmt-check:
	terraform fmt -check -recursive infrastructure/terraform/

build:
	cd backend && cargo build --workspace
	cd frontend && pnpm run build

deploy:
	scripts/deploy.sh
