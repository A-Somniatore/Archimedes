#!/bin/bash
# Test script for Python Native example
# Usage: ./test.sh [base_url]

set -e

BASE_URL="${1:-http://localhost:8002}"
PASSED=0
FAILED=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Helper function to run a test
run_test() {
    local name="$1"
    local expected_status="$2"
    shift 2
    local cmd=("$@")
    
    echo -n "Testing $name... "
    
    # Get response with status code on last line
    response=$(curl -s -w "\n%{http_code}" "${cmd[@]}")
    status=$(echo "$response" | tail -1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$status" -eq "$expected_status" ]; then
        echo -e "${GREEN}✓ PASSED${NC} (status: $status)"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAILED${NC} (expected: $expected_status, got: $status)"
        echo "  Response: $body"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Helper function for JSON body tests
test_json_field() {
    local name="$1"
    local expected_status="$2"
    local field="$3"
    local expected_value="$4"
    shift 4
    local cmd=("$@")
    
    echo -n "Testing $name... "
    
    response=$(curl -s -w "\n%{http_code}" "${cmd[@]}")
    status=$(echo "$response" | tail -1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$status" -ne "$expected_status" ]; then
        echo -e "${RED}✗ FAILED${NC} (expected status: $expected_status, got: $status)"
        FAILED=$((FAILED + 1))
        return 1
    fi
    
    actual_value=$(echo "$body" | jq -r "$field" 2>/dev/null || echo "null")
    
    if [ "$actual_value" = "$expected_value" ]; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAILED${NC} (expected $field=$expected_value, got: $actual_value)"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

echo "========================================"
echo "Archimedes Python Native Example Tests"
echo "========================================"
echo "Base URL: $BASE_URL"
echo ""

# Wait for server to be ready
echo -n "Waiting for server... "
for i in {1..10}; do
    if curl -s "$BASE_URL/health" > /dev/null 2>&1; then
        echo -e "${GREEN}ready${NC}"
        break
    fi
    if [ $i -eq 10 ]; then
        echo -e "${RED}timeout${NC}"
        echo "Server not responding at $BASE_URL"
        exit 1
    fi
    sleep 0.5
done

echo ""
echo "--- Health Check ---"
test_json_field "GET /health" 200 ".status" "healthy" "$BASE_URL/health"

echo ""
echo "--- List Users ---"
run_test "GET /users" 200 "$BASE_URL/users"
test_json_field "GET /users has total field" 200 ".total" "2" "$BASE_URL/users"

echo ""
echo "--- Get User ---"
run_test "GET /users/1" 200 "$BASE_URL/users/1"
test_json_field "GET /users/1 has correct name" 200 ".name" "Alice Smith" "$BASE_URL/users/1"
run_test "GET /users/nonexistent (should 404)" 404 "$BASE_URL/users/nonexistent"

echo ""
echo "--- Create User ---"
test_json_field "POST /users" 201 ".name" "Test User" \
    -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d '{"name":"Test User","email":"test@example.com"}'

# Check duplicate email (should fail)
run_test "POST /users duplicate email (should 400)" 400 \
    -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d '{"name":"Test User 2","email":"test@example.com"}'

echo ""
echo "--- Update User ---"
test_json_field "PUT /users/1" 200 ".name" "Alice Modified" \
    -X PUT "$BASE_URL/users/1" \
    -H "Content-Type: application/json" \
    -d '{"name":"Alice Modified"}'

run_test "PUT /users/nonexistent (should 404)" 404 \
    -X PUT "$BASE_URL/users/nonexistent" \
    -H "Content-Type: application/json" \
    -d '{"name":"Nobody"}'

echo ""
echo "--- Delete User ---"
run_test "DELETE /users/2" 204 -X DELETE "$BASE_URL/users/2"
run_test "DELETE /users/2 again (should 404)" 404 -X DELETE "$BASE_URL/users/2"

echo ""
echo "========================================"
echo "Test Results"
echo "========================================"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
