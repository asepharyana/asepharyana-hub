.PHONY: help dev lint format test clean update-submodules deploy init-submodules status

SHELL := /bin/bash

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

dev: ## Start development infrastructure (Redis etc.)
	docker compose -f infra/compose/shared.yml up -d

lint: ## Run Biome linter
	biome lint .

format: ## Format code with Biome
	biome format --write .

clean: ## Clean build artifacts
	rm -rf apps/*/dist apps/*/.next apps/*/target 2>/dev/null || true
	rm -rf node_modules 2>/dev/null || true

update-submodules: ## Update all git submodules to latest remote
	git submodule update --remote --merge --recursive

deploy: ## Deploy to VPS (triggers GitHub Actions)
	@echo "Push to main to trigger deployment, or run:"
	@echo "  gh workflow run deploy-docker.yml"

init-submodules: ## Initialize all submodules
	git submodule update --init --recursive

status: ## Show submodule status
	git submodule status
