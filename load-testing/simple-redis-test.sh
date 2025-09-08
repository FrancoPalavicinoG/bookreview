#!/bin/bash

# Simple test script for Redis-enabled deployments
# Usage: ./simple-redis-test.sh [cache|production] [user_count]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

show_usage() {
    echo "Usage: $0 [DEPLOYMENT_MODE] [USER_COUNT]"
    echo ""
    echo "DEPLOYMENT_MODE:"
    echo "  cache        Test cache deployment (app + database + redis)"
    echo "  production   Test production deployment (proxy + app + database + redis)"
    echo ""
    echo "USER_COUNT:"
    echo "  Number of concurrent users (default: 10)"
    echo ""
    echo "Examples:"
    echo "  $0 cache 5           # Test cache deployment with 5 users"
    echo "  $0 production 20     # Test production deployment with 20 users"
}

main() {
    local deployment_mode="${1:-cache}"
    local user_count="${2:-10}"
    
    # Validate deployment mode
    if [[ "$deployment_mode" != "cache" && "$deployment_mode" != "production" ]]; then
        error "Invalid deployment mode: $deployment_mode"
        show_usage
        exit 1
    fi
    
    # Validate user count
    if ! [[ "$user_count" =~ ^[0-9]+$ ]] || [[ "$user_count" -lt 1 ]]; then
        error "Invalid user count: $user_count"
        show_usage
        exit 1
    fi
    
    log "Starting simple test for $deployment_mode deployment with $user_count users"
    
    cd "$PROJECT_ROOT"
    
    # Start the appropriate deployment
    case $deployment_mode in
        "cache")
            log "Starting cache deployment (app + database + redis)..."
            docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml up -d --build
            local test_url="http://localhost:8000"
            local test_file="$SCRIPT_DIR/cache-deployment-test.jmx"
            ;;
        "production")
            log "Starting production deployment (proxy + app + database + redis)..."
            docker compose up -d --build
            local test_url="http://localhost"
            local test_file="$SCRIPT_DIR/proxy-deployment-test.jmx"
            ;;
    esac
    
    # Wait for services to be ready
    log "Waiting for services to be ready..."
    sleep 30
    
    # Health check
    local max_attempts=10
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -s -f "$test_url" > /dev/null 2>&1; then
            success "Deployment is ready"
            break
        fi
        
        log "Attempt $attempt/$max_attempts: Waiting for deployment..."
        sleep 10
        ((attempt++))
        
        if [[ $attempt -gt $max_attempts ]]; then
            error "Deployment failed to become ready"
            exit 1
        fi
    done
    
    # Seed the database
    log "Seeding database with sample data..."
    docker compose run --rm web sh -c '/app/seeder' || {
        warning "Seeder failed, but continuing with test..."
    }
    
    # Run the JMeter test
    log "Running JMeter test..."
    local results_file="/tmp/test_${deployment_mode}_${user_count}users.jtl"
    rm -f "$results_file"
    
    # Extract host and port from test_url
    local host="localhost"
    local port=""
    if [[ "$deployment_mode" == "cache" ]]; then
        port="8000"
    else
        port="80"
    fi
    
    if jmeter -n -t "$test_file" \
        -Jhost="$host" \
        -Jport="$port" \
        -Jthreads="$user_count" \
        -Jduration="60" \
        -l "$results_file"; then
        
        success "JMeter test completed successfully"
        
        # Show quick results
        if [[ -f "$results_file" ]]; then
            log "Quick Results Summary:"
            local total_requests=$(tail -n +2 "$results_file" | wc -l | tr -d ' ')
            local successful_requests=$(tail -n +2 "$results_file" | awk -F',' '$8=="true"' | wc -l | tr -d ' ')
            local avg_response_time=$(tail -n +2 "$results_file" | awk -F',' 'BEGIN{sum=0; count=0} {sum+=$2; count++} END{if(count>0) print sum/count; else print 0}')
            
            echo "  Total Requests: $total_requests"
            echo "  Successful Requests: $successful_requests"
            echo "  Average Response Time: ${avg_response_time}ms"
            
            if [[ "$total_requests" -gt 0 ]]; then
                local success_rate=$(echo "scale=2; ($successful_requests * 100) / $total_requests" | bc -l)
                echo "  Success Rate: ${success_rate}%"
            fi
        fi
    else
        error "JMeter test failed"
    fi
    
    # Show container stats
    log "Current container stats:"
    docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" | head -n 10
    
    # Cleanup
    log "Cleaning up..."
    case $deployment_mode in
        "cache")
            docker compose -f docker-compose.basic.yml -f docker-compose.cache.yml down
            ;;
        "production")
            docker compose down
            ;;
    esac
    
    success "Test completed successfully!"
}

# Check if help is requested
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    show_usage
    exit 0
fi

# Run main function
main "$@"
