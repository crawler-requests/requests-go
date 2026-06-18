#!/usr/bin/env bash
# requests-go CLI integration tests
set -euo pipefail

CLI="${CLI:-./requests-go}"
PASS=0
FAIL=0

assert_contains() {
    local desc="$1"
    local cmd="$2"
    local expected="$3"
    echo -n "  TEST: $desc ... "
    local output
    output=$(eval "$cmd" 2>&1) || true
    if echo "$output" | grep -q "$expected"; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL (expected '$expected' not found)"
        echo "       output: $output"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== requests-go CLI Integration Tests ==="
echo ""

# Build
echo "[Building]"
cd /home/cc11001100/github/crawler-requests/requests-go
go build -o "$CLI" ./cmd/requests-go/
echo ""

# 1. version
echo "[version]"
assert_contains "prints version" "$CLI version" "requests-go"

# 2. request — GET
echo "[request]"
assert_contains "GET httpbin.org/get" \
    "$CLI request --json '{\"method\":\"GET\",\"url\":\"https://httpbin.org/get\"}'" \
    '"status_code":200'

# 3. request — POST with JSON
echo "[request POST]"
assert_contains "POST httpbin.org/post" \
    "$CLI request --json '{\"method\":\"POST\",\"url\":\"https://httpbin.org/post\",\"json\":{\"hello\":\"world\"}}'" \
    '"status_code":200'

# 4. request — custom headers
echo "[request headers]"
assert_contains "custom headers" \
    "$CLI request --json '{\"method\":\"GET\",\"url\":\"https://httpbin.org/headers\",\"headers\":{\"X-Custom\":\"test123\"}}'" \
    '"status_code":200'

# 5. request — stdin input
echo "[request stdin]"
assert_contains "stdin input" \
    "echo '{\"method\":\"GET\",\"url\":\"https://httpbin.org/get\"}' | $CLI request" \
    '"status_code":200'

# 6. request — JA3 fingerprint
echo "[request JA3]"
assert_contains "JA3 fingerprint" \
    "$CLI request --ja3 '771,4865-4866-4867-49195-49199-49196-49200-52393-52392-49171-49172-156-157-47-53,0-23-65281-10-11-35-16-5-13-18-51-45-43-27-17513,29-23-24,0' --json '{\"method\":\"GET\",\"url\":\"https://httpbin.org/get\"}'" \
    '"status_code":200'

# 7. sessions / help
echo "[sessions/help]"
assert_contains "help shows session flag" "$CLI request --help" "session"

# 8. error handling
echo "[error handling]"
set +e
output=$($CLI request --json '{"method":"GET","url":"not-a-valid-url"}' 2>&1)
exit_code=$?
set -e
echo -n "  TEST: invalid URL returns error ... "
if [ $exit_code -ne 0 ]; then
    echo "PASS (exit code: $exit_code)"
    PASS=$((PASS + 1))
else
    echo "FAIL (expected non-zero exit code)"
    FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ]
