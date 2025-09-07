#!/bin/bash

# Simple JMeter Test Runner for BookReview
# Usage: ./simple-test.sh [basic|proxy] [1|10|100|1000|5000]

DEPLOYMENT_MODE=${1:-basic}
THREAD_COUNT=${2:-10}
TEST_DURATION=300  # 5 minutes

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"

mkdir -p "$RESULTS_DIR"

echo "Running JMeter test:"
echo "- Deployment: $DEPLOYMENT_MODE"
echo "- Users: $THREAD_COUNT"
echo "- Duration: $TEST_DURATION seconds (5 minutes)"

if [[ "$DEPLOYMENT_MODE" == "basic" ]]; then
    URL="http://localhost:8000"
    TEST_FILE="$SCRIPT_DIR/basic-deployment-test.jmx"
else
    URL="http://app.localhost"
    TEST_FILE="$SCRIPT_DIR/proxy-deployment-test.jmx"
fi

echo "- Target URL: $URL"

# Check if target is reachable
echo "Checking if application is reachable..."
if ! curl -s "$URL/health" &>/dev/null; then
    echo "ERROR: Application is not reachable at $URL"
    echo "Make sure the appropriate deployment is running:"
    if [[ "$DEPLOYMENT_MODE" == "basic" ]]; then
        echo "  docker compose -f docker-compose.basic.yml up -d --build"
    else
        echo "  docker compose up -d --build"
        echo "  And ensure 'app.localhost' is in /etc/hosts"
    fi
    exit 1
fi

echo "Application is reachable. Starting test..."

# Create a temporary test file with the correct thread count enabled
TEMP_TEST_FILE="$RESULTS_DIR/temp_${DEPLOYMENT_MODE}_${THREAD_COUNT}.jmx"
cp "$TEST_FILE" "$TEMP_TEST_FILE"

# Enable the correct thread group (this is a simplified approach)
# In practice, you might want to use JMeter properties or command line options

RESULTS_FILE="$RESULTS_DIR/${DEPLOYMENT_MODE}_${THREAD_COUNT}users_$(date +%Y%m%d_%H%M%S).jtl"
REPORT_DIR="$RESULTS_DIR/${DEPLOYMENT_MODE}_${THREAD_COUNT}users_report_$(date +%Y%m%d_%H%M%S)"

echo "Starting JMeter test..."
echo "Results will be saved to: $RESULTS_FILE"
echo "HTML report will be generated in: $REPORT_DIR"

jmeter -n -t "$TEMP_TEST_FILE" \
    -l "$RESULTS_FILE" \
    -e -o "$REPORT_DIR" \
    -Jthread_count="$THREAD_COUNT" \
    -Jtest_duration="$TEST_DURATION"

if [[ $? -eq 0 ]]; then
    echo "Test completed successfully!"
    echo "Results: $RESULTS_FILE"
    echo "Report: $REPORT_DIR/index.html"
    
    # Clean up temp file
    rm -f "$TEMP_TEST_FILE"
    
    # Show basic statistics
    echo ""
    echo "Quick Statistics:"
    if [[ -f "$RESULTS_FILE" ]]; then
        echo "Total Samples: $(tail -n +2 "$RESULTS_FILE" | wc -l)"
        echo "Success Rate: $(tail -n +2 "$RESULTS_FILE" | awk -F',' '{if($8=="true") success++; total++} END {printf "%.2f%%", (success/total)*100}')"
        echo "Average Response Time: $(tail -n +2 "$RESULTS_FILE" | awk -F',' '{sum+=$2; count++} END {printf "%.2f ms", sum/count}'))"
    fi
else
    echo "Test failed! Check the JMeter logs for details."
    exit 1
fi
