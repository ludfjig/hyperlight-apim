#!/usr/bin/env bash
# Drives the running gateway with a curl session. Start the gateway first
# with `just run` in another terminal.
set -uo pipefail

base="${GATEWAY_URL:-http://localhost:5000}"

# Fail early with a clear message if the gateway is not up yet.
if ! curl -s -o /dev/null --max-time 3 "$base/" -H "X-Customer: A"; then
    echo "error: no gateway reachable at $base" >&2
    echo "Start it first in another terminal with: just run" >&2
    exit 1
fi

hit() {
    local desc="$1"; shift
    local code
    code=$(curl -s -o /tmp/policy_demo_body -w '%{http_code}' "$@")
    printf '%-45s -> %s  %s\n' "$desc" "$code" "$(cat /tmp/policy_demo_body)"
}

echo "== Customer A: Rust auth-check =="
hit "A no auth header"        -H "X-Customer: A" "$base/orders/42"
hit "A with auth header"      -H "X-Customer: A" -H "Authorization: Bearer x" "$base/orders/42"

echo
echo "== Customer B: JS path-block =="
hit "B /products"             -H "X-Customer: B" "$base/products"
hit "B /admin/users"          -H "X-Customer: B" "$base/admin/users"
