# Changelog

All notable user-visible changes are recorded here.

## v0.2.0 - 2026-09-27

### Shelf

- A publish can replace the latest version: the fix keeps the version number and the old files are hidden (`shelf.sh publish --replace`).
- Versions can be retired (`shelf.sh retire`); retiring the latest lets the next publish reuse its number.

## v0.1.0 - 2026-09-23

### Shelf

- Added pieces with numbered versions of MusicXML, MIDI, audio and PDF files, visible once every file is uploaded and verified.
- Added the phone-first app: sign-in with an authenticator code, piece list, version history, downloads and hand-edit uploads.
- Added `scripts/shelf.sh` for the composition agent to publish, list and pull versions with a client-credentials token.
