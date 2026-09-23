#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TF_DIR="${ROOT_DIR}/infrastructure/terraform"
PLATFORM_BIN="${ROOT_DIR}/../ahara/bin"

if [ -d "${PLATFORM_BIN}" ]; then
  PATH="${PLATFORM_BIN}:${PATH}"
fi

STATE_BUCKET="${STATE_BUCKET:-tfstate-559098897826}"
STATE_REGION="${STATE_REGION:-us-east-1}"

echo "Building Lambda..."
cd "${ROOT_DIR}/backend"
cargo lambda build --release
cd "${ROOT_DIR}"

echo "Building frontend..."
cd "${ROOT_DIR}/frontend"
pnpm install --frozen-lockfile
pnpm run build

echo "Running database migrations..."
cd "${ROOT_DIR}"
db-migrate

echo "Deploying infrastructure..."
terraform -chdir="${TF_DIR}" init -reconfigure \
  -backend-config="bucket=${STATE_BUCKET}" \
  -backend-config="region=${STATE_REGION}" \
  -backend-config="use_lockfile=true"

terraform -chdir="${TF_DIR}" apply -auto-approve

echo ""
echo "Deploy complete."
echo "Frontend: $(terraform -chdir="${TF_DIR}" output -raw frontend_url)"
echo "API:      $(terraform -chdir="${TF_DIR}" output -raw api_url)"
