#!/usr/bin/env bash
# Publisher client for Score Shelf.
#
#   shelf.sh publish <slug> --label TEXT [--title TEXT] [--notes TEXT] [--ref TEXT] [--replace] FILE...
#   shelf.sh retire <slug> --version N
#   shelf.sh list
#   shelf.sh pull <slug> [--version N] [--out DIR]
#
# Publish a new version only for a meaningful musical revision. A fix to the
# latest version (an export repair, a re-tag with the commit ref) uses --replace:
# it takes over the latest version's number, and the old files are hidden.
# retire hides a version that should not have been published; retiring the
# latest lets the next publish reuse its number.
#
# Authenticates with the publisher's client-credentials grant. The client id
# and secret are created by ahara-infra (services/score-shelf-publisher.tf)
# and stored in SSM under /ahara/score-shelf/. The script reads them from
# SCORE_SHELF_CLIENT_ID / SCORE_SHELF_CLIENT_SECRET when set, otherwise from
# SSM with the ambient AWS credentials.
set -euo pipefail

API_URL="${SCORE_SHELF_API_URL:-https://api.score-shelf.ahara.io}"
TOKEN_URL="${SCORE_SHELF_TOKEN_URL:-https://auth.services.ahara.io/oauth2/token}"
SCOPE="${SCORE_SHELF_SCOPE:-score-shelf/publish}"
SSM_PREFIX="/ahara/score-shelf"

die() {
  echo "shelf.sh: $*" >&2
  exit 1
}

ssm_value() {
  aws ssm get-parameter --name "${SSM_PREFIX}/$1" --with-decryption \
    --query Parameter.Value --output text \
    || die "could not read ${SSM_PREFIX}/$1 from SSM; set SCORE_SHELF_CLIENT_ID/SECRET instead"
}

token() {
  # ssm_value runs in a subshell, so its die only ends that subshell; stop here too.
  if [ -z "${SCORE_SHELF_CLIENT_ID:-}" ]; then
    SCORE_SHELF_CLIENT_ID="$(ssm_value publisher-client-id)" || exit 1
  fi
  if [ -z "${SCORE_SHELF_CLIENT_SECRET:-}" ]; then
    SCORE_SHELF_CLIENT_SECRET="$(ssm_value publisher-client-secret)" || exit 1
  fi
  curl -fsS -u "${SCORE_SHELF_CLIENT_ID}:${SCORE_SHELF_CLIENT_SECRET}" \
    -d "grant_type=client_credentials" --data-urlencode "scope=${SCOPE}" \
    "${TOKEN_URL}" | jq -er .access_token
}

api() {
  local method="$1" path="$2" body="${3:-}"
  local args=(-sS -X "${method}" -H "Authorization: Bearer ${ACCESS_TOKEN}" -w '\n%{http_code}')
  if [ -n "${body}" ]; then
    args+=(-H "Content-Type: application/json" --data "${body}")
  fi
  local response status
  response="$(curl "${args[@]}" "${API_URL}${path}")"
  status="${response##*$'\n'}"
  response="${response%$'\n'*}"
  if [ "${status}" -ge 300 ]; then
    die "${method} ${path} failed (${status}): ${response}"
  fi
  printf '%s' "${response}"
}

kind_of() {
  case "${1,,}" in
    *.musicxml | *.xml | *.mxl) echo "musicxml application/vnd.recordare.musicxml+xml" ;;
    *.mid | *.midi) echo "midi audio/midi" ;;
    *.wav) echo "audio audio/wav" ;;
    *.mp3) echo "audio audio/mpeg" ;;
    *.flac) echo "audio audio/flac" ;;
    *.pdf) echo "pdf application/pdf" ;;
    *) echo "other application/octet-stream" ;;
  esac
}

file_entries() {
  local file kind content_type
  for file in "$@"; do
    [ -f "${file}" ] || die "no such file: ${file}"
    read -r kind content_type <<<"$(kind_of "${file}")"
    jq -n --arg name "$(basename "${file}")" --arg kind "${kind}" --arg type "${content_type}" \
      --argjson size "$(stat -c %s "${file}")" \
      '{filename: $name, kind: $kind, content_type: $type, size_bytes: $size}'
  done | jq -s .
}

publish() {
  local slug="${1:-}"
  [ -n "${slug}" ] || die "publish needs a piece slug"
  shift
  local label="" title="" notes="" ref="" replace=""
  local files=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --label) label="$2"; shift 2 ;;
      --title) title="$2"; shift 2 ;;
      --notes) notes="$2"; shift 2 ;;
      --ref) ref="$2"; shift 2 ;;
      --replace) replace="yes"; shift ;;
      *) files+=("$1"); shift ;;
    esac
  done
  [ -n "${label}" ] || die "publish needs --label"
  [ "${#files[@]}" -gt 0 ] || die "publish needs at least one file"

  local replaces="null"
  if [ -n "${replace}" ]; then
    replaces="$(api GET "/pieces/${slug}" | jq -e '.versions[0].number')" \
      || die "--replace needs an existing version of ${slug}"
  fi

  local body created version_id
  body="$(jq -n --arg label "${label}" --arg title "${title}" --arg notes "${notes}" \
    --arg ref "${ref}" --argjson replaces "${replaces}" \
    --argjson files "$(file_entries "${files[@]}")" \
    '{label: $label, notes: $notes, files: $files}
     + (if $title == "" then {} else {title: $title} end)
     + (if $ref == "" then {} else {source_ref: $ref} end)
     + (if $replaces == null then {} else {replaces: $replaces} end)')"
  created="$(api POST "/pieces/${slug}/versions" "${body}")"
  version_id="$(jq -er .version_id <<<"${created}")"

  local file name url
  for file in "${files[@]}"; do
    name="$(basename "${file}")"
    url="$(jq -er --arg name "${name}" '.uploads[] | select(.filename == $name) | .upload.url' \
      <<<"${created}")"
    read -r _ content_type <<<"$(kind_of "${file}")"
    curl -fsS -X PUT -H "content-type: ${content_type}" --upload-file "${file}" "${url}" \
      >/dev/null || die "upload of ${name} failed"
  done
  api POST "/versions/${version_id}/commit" >/dev/null
  if [ -n "${replace}" ]; then
    echo "Replaced ${slug} v$(jq -r .number <<<"${created}"): ${label}"
  else
    echo "Published ${slug} v$(jq -r .number <<<"${created}"): ${label}"
  fi
}

retire() {
  local slug="${1:-}" version=""
  [ -n "${slug}" ] || die "retire needs a piece slug"
  shift
  while [ $# -gt 0 ]; do
    case "$1" in
      --version) version="$2"; shift 2 ;;
      *) die "unknown option $1" ;;
    esac
  done
  [[ "${version}" =~ ^[0-9]+$ ]] || die "retire needs --version N"
  api POST "/pieces/${slug}/versions/${version}/retire" >/dev/null
  echo "Retired ${slug} v${version}"
}

list() {
  api GET /pieces | jq -r '.pieces[] |
    "\(.slug)\tv\(.latest.number)\t\(.latest.source)\t\(.latest.label)\t(\(.version_count) versions)"'
}

pull() {
  local slug="${1:-}" version="" out="."
  [ -n "${slug}" ] || die "pull needs a piece slug"
  shift
  while [ $# -gt 0 ]; do
    case "$1" in
      --version) version="$2"; shift 2 ;;
      --out) out="$2"; shift 2 ;;
      *) die "unknown option $1" ;;
    esac
  done
  local detail selected
  detail="$(api GET "/pieces/${slug}")"
  if [ -n "${version}" ]; then
    selected="$(jq -e --argjson n "${version}" '.versions[] | select(.number == $n)' \
      <<<"${detail}")" || die "no version ${version}"
  else
    selected="$(jq -e '.versions[0]' <<<"${detail}")"
  fi
  mkdir -p "${out}"
  local id name url
  while read -r id name; do
    url="$(api GET "/files/${id}/download" | jq -er .url)"
    curl -fsS -o "${out}/${name}" "${url}"
    echo "${out}/${name}"
  done < <(jq -r '.files[] | "\(.id) \(.filename)"' <<<"${selected}")
}

command="${1:-}"
[ -n "${command}" ] || die "usage: shelf.sh publish|retire|list|pull ..."
shift
ACCESS_TOKEN="$(token)"
case "${command}" in
  publish) publish "$@" ;;
  retire) retire "$@" ;;
  list) list ;;
  pull) pull "$@" ;;
  *) die "unknown command ${command}" ;;
esac
