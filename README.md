# Score Shelf

A private, versioned shelf for composition exports. The composition agent
publishes each MusicXML/MIDI export as a numbered version of a piece; the
owner browses and downloads versions on a phone and uploads hand edits back
as owner versions for the agent to pull.

## Architecture

| Part | What it is |
| --- | --- |
| Frontend | Vite + React SPA (`frontend/`), phone-first, Cognito sign-in with TOTP |
| API | One Rust Lambda (`backend/api`) behind the shared Ahara ALB |
| Data | Shared PostgreSQL (`score_shelf` database) for pieces, versions and files |
| Files | Private S3 bucket `score-shelf-files`; all transfers use presigned URLs |
| Auth | Shared Cognito pool: the app client for the owner, a client-credentials client (`score-shelf/publish`) for the agent |

Details: [docs/architecture.md](docs/architecture.md).

## URLs

| Surface | URL |
| --- | --- |
| App | `https://score-shelf.ahara.io` |
| API | `https://api.score-shelf.ahara.io` |

## Publishing from the dev box

```bash
with-cred -- scripts/shelf.sh publish boreal_pocket --label "key ladders" \
  --title "Boreal Pocket" --ref "$(git rev-parse --short HEAD)" \
  ../sigillum-explorations/outputs/explorations/boreal_pocket/boreal_pocket/*
with-cred -- scripts/shelf.sh list
with-cred -- scripts/shelf.sh pull boreal_pocket --out /tmp/boreal-edit
```

Outside Sulion, set `SCORE_SHELF_CLIENT_ID` and `SCORE_SHELF_CLIENT_SECRET`
(SSM `/ahara/score-shelf/publisher-client-id` and `publisher-client-secret`).

## Local development

```bash
cd frontend
pnpm install
pnpm dev          # http://localhost:5173, VITE_* values from .env
```

```bash
make ci           # lint, telemetry check, format, typecheck, tests, terraform fmt
make db-test      # PostgreSQL store tests in a container
```

## Deploy

CI deploys on push to `main` through the shared Ahara workflow. The local
equivalent is `source .env && scripts/deploy.sh`.

## License

MIT — see [LICENSE](LICENSE).
