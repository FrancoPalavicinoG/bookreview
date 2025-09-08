#!/bin/bash

# BookReview Load Testing Script with Redis Cache
# This script runs JMeter load tests for cache and production deployment modes

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

# Create results directories
mkdir -p "$RESULTS_DIR/cache"
mkdir -p "$RESULTS_DIR/production"

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
    local metrics_file="$RESULTS_DIR/${deployment_mode}/${deployment_mode}_${thread_count}users_metrics.txt"
    
    log "Starting system monitoring for $deployment_mode deployment with $thread_count users"
    
    # Create metrics file with headers
    echo "timestamp,cpu_percent,memory_mb,memory_percent,container_name,threads_count" > "$metrics_file"
    
    # Start monitoring in background
    while true; do
        timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        
        # Get container stats based on deployment mode
        if [[ "$deployment_mode" == "cache" ]]; then
            containers=("bookreview_web" "bookreview_mongo" "bookreview_redis")
        else
            containers=("bookreview_apache" "bookreview_web" "bookreview_mongo" "bookreview_redis")
        fi
        
        for container in "${containers[@]}"; do
            if docker ps --format "table {{.Names}}" | grep -q "$container"; then
                # Get container stats
                stats=$(docker stats --no-stream --format "table {{.CPUPerc}},{{.MemUsage}}" "$container" | tail -n 1)
                cpu_percent=$(echo "$stats" | cut -d',' -f1 | sed 's/%//' | tr -d ' ')
                memory_info=$(echo "$stats" | cut -d',' -f2 | tr -d ' ')
                
                # Parse memory usage
                memory_mb=""
                memory_percent=""
                
                if [[ "$memory_info" =~ ([0-9.]+)([MG])iB/([0-9.]+)([MG])iB ]]; then
                    used_value="${BASH_REMATCH[1]}"
                    used_unit="${BASH_REMATCH[2]}"
                    total_value="${BASH_REMATCH[3]}"
                    total_unit="${BASH_REMATCH[4]}"
                    
                    # Convert to MB
                    if [[ "$used_unit" == "G" ]]; then
                        memory_mb=$(echo "$used_value * 1024" | bc -l)
                    else
                        memory_mb="$used_value"
                    fi
                    
                    if [[ "$total_unit" == "G" ]]; then
                        total_mb=$(echo "$total_value * 1024" | bc -l)
                    else
                        total_mb="$total_value"
                    fi
                    
                    # Calculate percentage
                    memory_percent=$(echo "scale=2; ($memory_mb / $total_mb) * 100" | bc -l)
                fi
                
                # Get thread count for the container
                threads_count=""
                if [[ "$container" == "bookreview_web" ]]; then
                    # Get number of threads for the Rust application
                    pid=$(docker exec "$container" pgrep -f bookreview 2>/dev/null || echo "")
                    if [[ -n "$pid" ]]; then
                        threads_count=$(docker exec "$container" cat /proc/$pid/status 2>/dev/null | grep "Threads:" | awk '{print $2}' || echo "")
                    fi
                elif [[ "$container" == "bookreview_apache" ]]; then
                    # Get Apache process count
                    threads_count=$(docker exec "$container" pgrep httpd 2>/dev/null | wc -l || echo "")
                elif [[ "$container" == "bookreview_mongo" ]]; then
                    # Get MongoDB connection count
                    threads_count=$(docker exec "$container" mongo --quiet --eval "db.serverStatus().connections.current" 2>/dev/null || echo "")
                elif [[ "$container" == "bookreview_redis" ]]; then
                    # Get Redis connected clients
                    threads_count=$(docker exec "$container" redis-cli info clients 2>/dev/null | grep "connected_clients:" | cut -d: -f2 | tr -d '\r' || echo "")
                fi
                
                # Clean up values
                memory_mb=$(echo "$memory_mb" | cut -d'.' -f1)
                memory_percent=$(echo "$memory_percent" | cut -d'.' -f1)
                threads_count=${threads_count:-"0"}
                
                echo "$timestamp,$cpu_percent,$memory_mb,$memory_percent,$container,$threads_count" >> "$metrics_file"
            fi
        done
        
        sleep 5
    done &
    
    # Store monitoring PID
    echo $! > "/tmp/monitoring_${deployment_mode}_${thread_count}.pid"
}

stop_monitoring() {
    local deployment_mode=$1
    local thread_count=$2
    local pid_file="/tmp/monitoring_${deployment_mode}_${thread_count}.pid"
    
    if [[ -f "$pid_file" ]]; then
        local monitoring_pid=$(cat "$pid_file")
        if kill -0 "$monitoring_pid" 2>/dev/null; then
            log "Stopping monitoring (PID: $monitoring_pid)"
            kill "$monitoring_pid"
        fi
        rm -f "$pid_file"
    fi
}

start_deployment() {
    local mode=$1
    
    log "Starting $mode deployment..."
    
    cd "$PROJECT_ROOT"
    
    case $mode in
        "cache")
            # Start app + database + redis (without proxy)
            if ! docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml up -d; then
                error "Failed to start cache deployment"
                return 1
            fi
            ;;
        "production")
            # Start full production setup (proxy + app + database + redis)
            if ! docker compose up -d; then
                error "Failed to start production deployment"
                return 1
            fi
            ;;
        *)
            error "Unknown deployment mode: $mode"
            return 1
            ;;
    esac
    
    log "Waiting for services to be ready..."
    sleep 30
    
    # Health check
    local health_url=""
    case $mode in
        "cache")
            health_url="http://localhost:8000"
            ;;
        "production")
            health_url="http://localhost:80"
            ;;
    esac
    
    local max_attempts=12
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -s -f "$health_url" > /dev/null 2>&1; then
            success "$mode deployment is ready"
            return 0
        fi
        
        log "Attempt $attempt/$max_attempts: Waiting for $mode deployment..."
        sleep 10
        ((attempt++))
    done
    
    error "$mode deployment failed to become ready"
    return 1
}

stop_deployment() {
    local mode=$1
    
    log "Stopping $mode deployment..."
    
    cd "$PROJECT_ROOT"
    
    case $mode in
        "cache")
            docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml down
            ;;
        "production")
            docker compose down
            ;;
    esac
    
    # Clean up any remaining containers
    docker container prune -f > /dev/null 2>&1 || true
    
    success "$mode deployment stopped"
}

run_load_test() {
    local mode=$1
    local thread_count=$2
    
    log "Running load test for $mode deployment with $thread_count users"
    
    local test_file=""
    local host="localhost"
    local port=""
    local results_file="$RESULTS_DIR/${mode}/${mode}_${thread_count}users_jmeter.jtl"
    
    case $mode in
        "cache")
            test_file="$SCRIPT_DIR/cache-deployment-test.jmx"
            port="8000"
            ;;
        "production")
            test_file="$SCRIPT_DIR/proxy-deployment-test.jmx"
            port="80"
            ;;
    esac
    
    # Remove existing results file
    rm -f "$results_file"
    
    # Start monitoring
    start_monitoring "$mode" "$thread_count"
    
    # Run JMeter test
    if jmeter -n -t "$test_file" \
        -Jhost="$host" \
        -Jport="$port" \
        -Jthreads="$thread_count" \
        -Jduration="$TEST_DURATION" \
        -l "$results_file" \
        -e -o "$RESULTS_DIR/${mode}/${mode}_${thread_count}users_report"; then
        
        success "Load test completed for $mode deployment with $thread_count users"
    else
        error "Load test failed for $mode deployment with $thread_count users"
    fi
    
    # Stop monitoring
    stop_monitoring "$mode" "$thread_count"
    
    # Generate summary
    generate_summary "$mode" "$thread_count" "$results_file"
}

generate_summary() {
    local mode=$1
    local thread_count=$2
    local results_file=$3
    local summary_file="$RESULTS_DIR/${mode}/${mode}_${thread_count}users_summary.txt"
    
    if [[ ! -f "$results_file" ]]; then
        warning "Results file not found: $results_file"
        return
    fi
    
    log "Generating summary for $mode deployment with $thread_count users"
    
    # Calculate statistics from JMeter results
    awk -F',' '
    BEGIN {
        total_requests = 0
        successful_requests = 0
        total_response_time = 0
        min_response_time = 999999
        max_response_time = 0
        response_codes[200] = 0
        response_codes[404] = 0
        response_codes[500] = 0
        response_codes["other"] = 0
    }
    NR > 1 {  # Skip header
        total_requests++
        response_time = $2
        success = $8
        response_code = $4
        
        if (success == "true") {
            successful_requests++
        }
        
        total_response_time += response_time
        if (response_time < min_response_time) min_response_time = response_time
        if (response_time > max_response_time) max_response_time = response_time
        
        if (response_code == 200) {
            response_codes[200]++
        } else if (response_code == 404) {
            response_codes[404]++
        } else if (response_code == 500) {
            response_codes[500]++
        } else {
            response_codes["other"]++
        }
    }
    END {
        if (total_requests > 0) {
            success_rate = (successful_requests / total_requests) * 100
            avg_response_time = total_response_time / total_requests
            throughput = total_requests / 300  # 5 minutes
            
            print "=== Load Test Summary ==="
            print "Deployment Mode: " mode
            print "Thread Count: " thread_count
            print "Test Duration: 5 minutes"
            print ""
            print "=== Request Statistics ==="
            print "Total Requests: " total_requests
            print "Successful Requests: " successful_requests
            print "Success Rate: " sprintf("%.2f%%", success_rate)
            print "Throughput: " sprintf("%.2f requests/second", throughput)
            print ""
            print "=== Response Time Statistics (ms) ==="
            print "Average: " sprintf("%.2f", avg_response_time)
            print "Minimum: " min_response_time
            print "Maximum: " max_response_time
            print ""
            print "=== Response Codes ==="
            print "200 (OK): " response_codes[200]
            print "404 (Not Found): " response_codes[404]
            print "500 (Server Error): " response_codes[500]
            print "Other: " response_codes["other"]
        }
    }' mode="$mode" thread_count="$thread_count" "$results_file" > "$summary_file"
    
    # Display summary
    cat "$summary_file"
}

run_deployment_tests() {
    local mode=$1
    
    log "Starting tests for $mode deployment"
    
    if ! start_deployment "$mode"; then
        error "Failed to start $mode deployment"
        return 1
    fi
    
    for thread_count in "${THREAD_COUNTS[@]}"; do
        log "Running test with $thread_count users for $mode deployment"
        run_load_test "$mode" "$thread_count"
        
        # Wait between tests
        if [[ "$thread_count" != "${THREAD_COUNTS[-1]}" ]]; then
            log "Waiting 30 seconds before next test..."
            sleep 30
        fi
    done
    
    stop_deployment "$mode"
    success "Completed all tests for $mode deployment"
}

cleanup() {
    log "Cleaning up..."
    
    # Stop any running deployments
    cd "$PROJECT_ROOT"
    docker compose down > /dev/null 2>&1 || true
    docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml down > /dev/null 2>&1 || true
    
    # Kill any monitoring processes
    for pid_file in /tmp/monitoring_*.pid; do
        if [[ -f "$pid_file" ]]; then
            local pid=$(cat "$pid_file")
            kill "$pid" 2>/dev/null || true
            rm -f "$pid_file"
        fi
    done
    
    success "Cleanup completed"
}

show_usage() {
    echo "Usage: $0 [OPTIONS] [DEPLOYMENT_MODE]"
    echo ""
    echo "DEPLOYMENT_MODE:"
    echo "  cache        Run tests for cache deployment (app + database + redis)"
    echo "  production   Run tests for production deployment (proxy + app + database + redis)"
    echo "  all          Run tests for all deployment modes (default)"
    echo ""
    echo "OPTIONS:"
    echo "  -h, --help   Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 cache                    # Test cache deployment only"
    echo "  $0 production              # Test production deployment only"
    echo "  $0                         # Test all deployments"
}

main() {
    local deployment_mode="all"
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            cache|production|all)
                deployment_mode="$1"
                shift
                ;;
            *)
                error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Set up signal handlers
    trap cleanup EXIT INT TERM
    
    # Check dependencies
    check_dependencies
    
    log "Starting BookReview load testing with Redis cache"
    log "Test configuration: ${THREAD_COUNTS[*]} users, $TEST_DURATION seconds each"
    log "Results will be saved to: $RESULTS_DIR"
    
    case $deployment_mode in
        "cache")
            run_deployment_tests "cache"
            ;;
        "production")
            run_deployment_tests "production"
            ;;
        "all")
            run_deployment_tests "cache"
            log "Waiting 60 seconds between deployment modes..."
            sleep 60
            run_deployment_tests "production"
            ;;
    esac
    
    success "All load tests completed!"
    log "Results are available in: $RESULTS_DIR"
}

# Run main function
main "$@"
