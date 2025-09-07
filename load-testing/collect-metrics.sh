#!/bin/bash

# Docker Container Metrics Collection Script
# Usage: ./collect-metrics.sh [basic|proxy] [duration_in_seconds]

DEPLOYMENT_MODE=${1:-basic}
DURATION=${2:-300}  # 5 minutes default
INTERVAL=5  # Collect metrics every 5 seconds

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
METRICS_FILE="$RESULTS_DIR/${DEPLOYMENT_MODE}_metrics_${TIMESTAMP}.csv"

mkdir -p "$RESULTS_DIR"

echo "Collecting Docker container metrics for $DEPLOYMENT_MODE deployment"
echo "Duration: $DURATION seconds"
echo "Interval: $INTERVAL seconds"
echo "Output file: $METRICS_FILE"

# Determine which containers to monitor
if [[ "$DEPLOYMENT_MODE" == "basic" ]]; then
    CONTAINERS=("bookreview_web" "bookreview_mongo")
else
    CONTAINERS=("bookreview_apache" "bookreview_web" "bookreview_mongo")
fi

# Create CSV header
echo "timestamp,container_name,cpu_percent,memory_usage_mb,memory_limit_mb,memory_percent,network_rx_mb,network_tx_mb,block_read_mb,block_write_mb,pids" > "$METRICS_FILE"

echo "Monitoring containers: ${CONTAINERS[*]}"
echo "Starting collection... (Press Ctrl+C to stop early)"

START_TIME=$(date +%s)
END_TIME=$((START_TIME + DURATION))

collect_metrics() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    for container in "${CONTAINERS[@]}"; do
        if docker ps --format "table {{.Names}}" | grep -q "^$container$"; then
            # Get detailed stats
            stats=$(docker stats --no-stream --format "{{.CPUPerc}},{{.MemUsage}},{{.NetIO}},{{.BlockIO}},{{.PIDs}}" "$container")
            
            if [[ -n "$stats" ]]; then
                # Parse stats
                IFS=',' read -r cpu_raw memory_raw network_raw block_raw pids <<< "$stats"
                
                # Clean CPU percentage
                cpu_percent=$(echo "$cpu_raw" | sed 's/%//')
                
                # Parse memory (format: 123.4MiB / 456.7MiB)
                memory_usage=$(echo "$memory_raw" | cut -d'/' -f1 | sed 's/MiB//' | sed 's/GiB/*1024/' | sed 's/ //g')
                memory_limit=$(echo "$memory_raw" | cut -d'/' -f2 | sed 's/MiB//' | sed 's/GiB/*1024/' | sed 's/ //g')
                
                # Calculate memory usage in MB
                memory_usage_mb=$(echo "$memory_usage" | bc -l 2>/dev/null || echo "0")
                memory_limit_mb=$(echo "$memory_limit" | bc -l 2>/dev/null || echo "0")
                
                # Calculate memory percentage
                if [[ "$memory_limit_mb" != "0" ]]; then
                    memory_percent=$(echo "scale=2; ($memory_usage_mb / $memory_limit_mb) * 100" | bc -l 2>/dev/null || echo "0")
                else
                    memory_percent="0"
                fi
                
                # Parse network I/O (format: 123kB / 456kB)
                network_rx=$(echo "$network_raw" | cut -d'/' -f1 | sed 's/kB//' | sed 's/MB/*1000/' | sed 's/GB/*1000000/' | sed 's/ //g')
                network_tx=$(echo "$network_raw" | cut -d'/' -f2 | sed 's/kB//' | sed 's/MB/*1000/' | sed 's/GB/*1000000/' | sed 's/ //g')
                
                network_rx_mb=$(echo "scale=3; $network_rx / 1000" | bc -l 2>/dev/null || echo "0")
                network_tx_mb=$(echo "scale=3; $network_tx / 1000" | bc -l 2>/dev/null || echo "0")
                
                # Parse block I/O (format: 123kB / 456kB)
                block_read=$(echo "$block_raw" | cut -d'/' -f1 | sed 's/kB//' | sed 's/MB/*1000/' | sed 's/GB/*1000000/' | sed 's/ //g')
                block_write=$(echo "$block_raw" | cut -d'/' -f2 | sed 's/kB//' | sed 's/MB/*1000/' | sed 's/GB/*1000000/' | sed 's/ //g')
                
                block_read_mb=$(echo "scale=3; $block_read / 1000" | bc -l 2>/dev/null || echo "0")
                block_write_mb=$(echo "scale=3; $block_write / 1000" | bc -l 2>/dev/null || echo "0")
                
                # Write to CSV
                echo "$timestamp,$container,$cpu_percent,$memory_usage_mb,$memory_limit_mb,$memory_percent,$network_rx_mb,$network_tx_mb,$block_read_mb,$block_write_mb,$pids" >> "$METRICS_FILE"
            fi
        else
            echo "$timestamp,$container,OFFLINE,0,0,0,0,0,0,0,0" >> "$METRICS_FILE"
        fi
    done
}

# Trap Ctrl+C to clean up
trap 'echo -e "\nCollection stopped by user"; exit 0' INT

# Main collection loop
while [[ $(date +%s) -lt $END_TIME ]]; do
    collect_metrics
    echo -ne "\rCollected metrics at $(date '+%H:%M:%S') - Remaining: $((END_TIME - $(date +%s)))s"
    sleep $INTERVAL
done

echo -e "\nMetrics collection completed!"
echo "Results saved to: $METRICS_FILE"

# Generate a quick summary
echo ""
echo "Quick Summary:"
echo "=============="

# Show container statistics
for container in "${CONTAINERS[@]}"; do
    echo ""
    echo "Container: $container"
    
    # Average CPU
    avg_cpu=$(grep "$container" "$METRICS_FILE" | grep -v "OFFLINE" | awk -F',' '{sum+=$3; count++} END {if(count>0) printf "%.2f", sum/count; else print "N/A"}')
    echo "  Average CPU: ${avg_cpu}%"
    
    # Max CPU  
    max_cpu=$(grep "$container" "$METRICS_FILE" | grep -v "OFFLINE" | awk -F',' 'BEGIN{max=0} {if($3>max) max=$3} END {printf "%.2f", max}')
    echo "  Max CPU: ${max_cpu}%"
    
    # Average Memory
    avg_memory=$(grep "$container" "$METRICS_FILE" | grep -v "OFFLINE" | awk -F',' '{sum+=$4; count++} END {if(count>0) printf "%.2f", sum/count; else print "N/A"}')
    echo "  Average Memory: ${avg_memory} MB"
    
    # Max Memory
    max_memory=$(grep "$container" "$METRICS_FILE" | grep -v "OFFLINE" | awk -F',' 'BEGIN{max=0} {if($4>max) max=$4} END {printf "%.2f", max}')
    echo "  Max Memory: ${max_memory} MB"
done

echo ""
echo "Detailed metrics available in: $METRICS_FILE"
