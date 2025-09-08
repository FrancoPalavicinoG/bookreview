#!/bin/bash

# BookReview Load Testing Script
# This script runs JMeter load tests for both deployment modes and collects system metrics

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$SCRIPT_DIR/results"
JMETER_HOME="${JMETER_HOME:-/usr/local/opt/jmeter}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configurations
declare -a THREAD_COUNTS=(1 10 100 1000 5000)
TEST_DURATION=300  # 5 minutes in seconds

# Create results directory
mkdir -p "$RESULTS_DIR"

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

check_dependencies() {
    log "Checking dependencies..."
    
    # Check if JMeter is installed
    if ! command -v jmeter &> /dev/null; then
        error "JMeter is not installed or not in PATH"
        echo "Please install JMeter:"
        echo "  macOS: brew install jmeter"
        echo "  Linux: Download from https://jmeter.apache.org/"
        exit 1
    fi
    
    # Check if Docker is running
    if ! docker info &> /dev/null; then
        error "Docker is not running"
        exit 1
    fi
    
    # Check if docker-compose is available
    if ! command -v docker &> /dev/null; then
        error "Docker Compose is not installed"
        exit 1
    fi
    
    success "All dependencies are available"
}

start_monitoring() {
    local deployment_mode=$1
    local thread_count=$2
    local metrics_file="$RESULTS_DIR/${deployment_mode}_${thread_count}users_metrics.txt"
    
    log "Starting system monitoring for $deployment_mode deployment with $thread_count users"
    
    # Create metrics file with headers
    echo "timestamp,cpu_percent,memory_mb,memory_percent,container_name" > "$metrics_file"
    
    # Start monitoring in background
    while true; do
        timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        
        # Get container stats
        if [[ "$deployment_mode" == "basic" ]]; then
            containers=("bookreview_web" "bookreview_mongo")
        else
            containers=("bookreview_apache" "bookreview_web" "bookreview_mongo")
        fi
        
        for container in "${containers[@]}"; do
            if docker ps --format "table {{.Names}}" | grep -q "$container"; then
                stats=$(docker stats --no-stream --format "table {{.CPUPerc}},{{.MemUsage}}" "$container" | tail -n 1)
                cpu_percent=$(echo "$stats" | cut -d',' -f1 | sed 's/%//')
                memory_info=$(echo "$stats" | cut -d',' -f2)
                memory_mb=$(echo "$memory_info" | cut -d'/' -f1 | sed 's/MiB//' | sed 's/GiB/*1024/' | bc -l 2>/dev/null || echo "0")
                memory_total=$(echo "$memory_info" | cut -d'/' -f2 | sed 's/GiB//' | sed 's/MiB//')
                
                # Calculate memory percentage (simplified)
                if [[ "$memory_info" == *"GiB"* ]]; then
                    memory_percent=$(echo "scale=2; ($memory_mb / ($memory_total * 1024)) * 100" | bc -l 2>/dev/null || echo "0")
                else
                    memory_percent=$(echo "scale=2; ($memory_mb / $memory_total) * 100" | bc -l 2>/dev/null || echo "0")
                fi
                
                echo "$timestamp,$cpu_percent,$memory_mb,$memory_percent,$container" >> "$metrics_file"
            fi
        done
        
        sleep 10
    done &
    
    echo $! > "$RESULTS_DIR/monitoring_${deployment_mode}_${thread_count}.pid"
}

stop_monitoring() {
    local deployment_mode=$1
    local thread_count=$2
    local pid_file="$RESULTS_DIR/monitoring_${deployment_mode}_${thread_count}.pid"
    
    if [[ -f "$pid_file" ]]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            log "Stopped monitoring process (PID: $pid)"
        fi
        rm -f "$pid_file"
    fi
}

run_jmeter_test() {
    local deployment_mode=$1
    local thread_count=$2
    local test_file="$SCRIPT_DIR/${deployment_mode}-deployment-test.jmx"
    local results_file="$RESULTS_DIR/${deployment_mode}_${thread_count}users_results.jtl"
    local summary_file="$RESULTS_DIR/${deployment_mode}_${thread_count}users_summary.csv"
    
    log "Running JMeter test: $deployment_mode deployment with $thread_count users"
    
    # Enable the appropriate thread group and disable others
    jmeter -n -t "$test_file" \
        -J "thread_count=$thread_count" \
        -J "test_duration=$TEST_DURATION" \
        -l "$results_file" \
        -e -o "$RESULTS_DIR/${deployment_mode}_${thread_count}users_report" \
        2>&1 | tee "$RESULTS_DIR/${deployment_mode}_${thread_count}users_jmeter.log"
    
    success "JMeter test completed for $deployment_mode deployment with $thread_count users"
}

setup_basic_deployment() {
    log "Setting up basic deployment (app + database)"
    cd "$PROJECT_ROOT"
    
    # Stop any running containers
    docker compose down -v 2>/dev/null || true
    docker compose -f docker-compose.basic.yml down -v 2>/dev/null || true
    
    # Start basic deployment
    docker compose -f docker-compose.basic.yml up -d --build
    
    # Wait for services to be ready
    log "Waiting for services to be ready..."
    timeout=60
    while ! curl -s http://localhost:8000/health &>/dev/null; do
        sleep 2
        timeout=$((timeout - 2))
        if [[ $timeout -le 0 ]]; then
            error "Basic deployment failed to start"
            docker compose -f docker-compose.basic.yml logs
            exit 1
        fi
    done
    
    # Load sample data
    log "Loading sample data..."
    docker compose -f docker-compose.basic.yml run --rm web sh -lc '/app/seeder' || warning "Seeder failed, continuing anyway"
    
    success "Basic deployment is ready"
}

setup_proxy_deployment() {
    log "Setting up proxy deployment (apache + app + database)"
    cd "$PROJECT_ROOT"
    
    # Stop any running containers
    docker compose down -v 2>/dev/null || true
    docker compose -f docker-compose.basic.yml down -v 2>/dev/null || true
    
    # Ensure hosts file entry exists
    if ! grep -q "app.localhost" /etc/hosts; then
        warning "Adding app.localhost to /etc/hosts (requires sudo)"
        echo "127.0.0.1 app.localhost" | sudo tee -a /etc/hosts
    fi
    
    # Start proxy deployment
    docker compose up -d --build
    
    # Wait for services to be ready
    log "Waiting for services to be ready..."
    timeout=60
    while ! curl -s http://app.localhost/health &>/dev/null; do
        sleep 2
        timeout=$((timeout - 2))
        if [[ $timeout -le 0 ]]; then
            error "Proxy deployment failed to start"
            docker compose logs
            exit 1
        fi
    done
    
    # Load sample data
    log "Loading sample data..."
    docker compose run --rm web sh -lc '/app/seeder' || warning "Seeder failed, continuing anyway"
    
    success "Proxy deployment is ready"
}

run_tests_for_deployment() {
    local deployment_mode=$1
    
    log "Starting tests for $deployment_mode deployment"
    
    for thread_count in "${THREAD_COUNTS[@]}"; do
        log "Testing with $thread_count concurrent users"
        
        # Start monitoring
        start_monitoring "$deployment_mode" "$thread_count"
        
        # Wait a moment for monitoring to start
        sleep 5
        
        # Run JMeter test
        run_jmeter_test "$deployment_mode" "$thread_count"
        
        # Stop monitoring
        stop_monitoring "$deployment_mode" "$thread_count"
        
        # Wait between tests
        if [[ "$thread_count" != "5000" ]]; then
            log "Waiting 30 seconds before next test..."
            sleep 30
        fi
    done
    
    success "All tests completed for $deployment_mode deployment"
}

generate_report() {
    log "Generating comparison report..."
    
    cat > "$RESULTS_DIR/load_test_report.md" << 'EOF'
# BookReview Load Testing Report

## Test Configuration
- **Test Duration**: 5 minutes per test
- **Thread Counts**: 1, 10, 100, 1000, 5000 users
- **Endpoints Tested**: /health, /, /books, /authors
- **Test Date**: $(date)

## Deployment Modes Tested

### Basic Deployment
- **Architecture**: Rust Web Application + MongoDB
- **URL**: http://localhost:8000
- **Static Files**: Served by Rust application

### Proxy Deployment  
- **Architecture**: Apache Reverse Proxy + Rust Web Application + MongoDB
- **URL**: http://app.localhost
- **Static Files**: Served by Apache

## Results Summary

### Response Times (Average)

| Users | Basic Deployment (ms) | Proxy Deployment (ms) | Difference |
|-------|----------------------|----------------------|------------|
EOF

    for thread_count in "${THREAD_COUNTS[@]}"; do
        basic_avg="N/A"
        proxy_avg="N/A"
        
        # Extract average response times (you'll need to parse JTL files)
        if [[ -f "$RESULTS_DIR/basic_${thread_count}users_results.jtl" ]]; then
            basic_avg=$(awk -F',' 'NR>1 {sum+=$2; count++} END {if(count>0) printf "%.2f", sum/count}' "$RESULTS_DIR/basic_${thread_count}users_results.jtl" 2>/dev/null || echo "N/A")
        fi
        
        if [[ -f "$RESULTS_DIR/proxy_${thread_count}users_results.jtl" ]]; then
            proxy_avg=$(awk -F',' 'NR>1 {sum+=$2; count++} END {if(count>0) printf "%.2f", sum/count}' "$RESULTS_DIR/proxy_${thread_count}users_results.jtl" 2>/dev/null || echo "N/A")
        fi
        
        echo "| $thread_count | $basic_avg | $proxy_avg | TBD |" >> "$RESULTS_DIR/load_test_report.md"
    done
    
    cat >> "$RESULTS_DIR/load_test_report.md" << 'EOF'

### System Resource Usage

Detailed metrics are available in the individual CSV files:
- CPU usage percentages
- Memory usage in MB
- Memory usage percentages
- Per-container breakdowns

### Files Generated

- `*_results.jtl` - Raw JMeter results
- `*_metrics.txt` - System resource metrics
- `*_report/` - JMeter HTML reports
- `*_jmeter.log` - JMeter execution logs

### Analysis

[Add your analysis here after reviewing the results]

EOF

    success "Report generated at $RESULTS_DIR/load_test_report.md"
}

main() {
    log "Starting BookReview Load Testing Suite"
    
    check_dependencies
    
    # Test basic deployment
    log "=== TESTING BASIC DEPLOYMENT ==="
    setup_basic_deployment
    run_tests_for_deployment "basic"
    
    # Test proxy deployment
    log "=== TESTING PROXY DEPLOYMENT ==="
    setup_proxy_deployment
    run_tests_for_deployment "proxy"
    
    # Clean up
    log "Cleaning up..."
    cd "$PROJECT_ROOT"
    docker compose down -v 2>/dev/null || true
    docker compose -f docker-compose.basic.yml down -v 2>/dev/null || true
    
    # Generate report
    generate_report
    
    success "Load testing completed! Results are in: $RESULTS_DIR"
    log "Review the generated report: $RESULTS_DIR/load_test_report.md"
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
