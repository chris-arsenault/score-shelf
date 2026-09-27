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
scripts/shelf.sh publish boreal_pocket --label "key ladders" \
  --title "Boreal Pocket" --ref "$(git rev-parse --short HEAD)" \
  ../sigillum-explorations/outputs/explorations/boreal_pocket/boreal_pocket/*
scripts/shelf.sh publish boreal_pocket --replace --label "key ladders" \
  --ref "$(git rev-parse --short HEAD)" \
  ../sigillum-explorations/outputs/explorations/boreal_pocket/boreal_pocket/*
scripts/shelf.sh retire boreal_pocket --version 4
scripts/shelf.sh list
scripts/shelf.sh pull boreal_pocket --out /tmp/boreal-edit
```

The shelf holds meaningful revisions only. Publish a new version for a
musical change. For a fix to the latest version (an export repair, or
re-tagging it with its commit), use `--replace`: the fix takes over the latest
version's number and the old files are hidden. `retire` hides a version that
should not have been published.

The publisher client is created by `ahara-infra`
(`services/score-shelf-publisher.tf`); its id and secret are in SSM at
`/ahara/score-shelf/publisher-client-id` and `publisher-client-secret`. The
script reads them from SSM with the ambient AWS credentials, or from
`SCORE_SHELF_CLIENT_ID` / `SCORE_SHELF_CLIENT_SECRET` when those are set.

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
